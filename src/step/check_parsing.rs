//! Parsing output from check commands.
//!
//! This module handles parsing the output of `check_list_files` and `check_diff`
//! commands to extract the list of files that need to be fixed.
//!
//! - `check_list_files`: Outputs one file path per line
//! - `check_diff`: Outputs unified diff format, files extracted from `---` and `+++` lines

use indexmap::IndexSet;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use xx::file::display_path;

use super::types::Step;
use super::{header_pairs, header_path, normalize_diff_paths, strips_git_prefixes};

pub(crate) struct DiffFiles {
    pub files: Vec<PathBuf>,
    pub created: Vec<PathBuf>,
    pub extras: Vec<PathBuf>,
}

/// Resolve a path from check command stdout to the job file(s) it names.
///
/// When a step runs with `dir` configured (such as in a subproject), commands
/// typically output paths relative to their working directory (`dir`).
///
/// If `dir` is set:
/// 1. If `original_files` contains `dir/p`, `p`, or both, every one it
///    contains is a match: a job can hold both `src/App.ts` and
///    `frontend/src/App.ts`, and a report of `src/App.ts` must not silently
///    drop the root file in favor of the subproject one (or vice versa).
/// 2. Otherwise, if `dir/p` exists on disk, use it (even if a file with the
///    same relative path exists at the repo root).
/// 3. Otherwise, if `p` exists on disk at the repo root, use it.
/// 4. Default to `p` because unrecognized paths should be reported as output by the tool.
fn resolve_path(raw: &str, dir: Option<&str>, original_files: &HashSet<&Path>) -> Vec<PathBuf> {
    let p = Path::new(raw);
    if p.as_os_str().is_empty() {
        return vec![PathBuf::new()];
    }
    let p = p.strip_prefix("./").unwrap_or(p);
    if p.is_absolute() {
        return vec![p.to_path_buf()];
    }
    let Some(dir) = dir.filter(|d| !d.is_empty() && *d != ".") else {
        return vec![p.to_path_buf()];
    };

    let in_dir = Path::new(dir).join(p);

    let matches: Vec<PathBuf> = [in_dir.as_path(), p]
        .into_iter()
        .filter(|candidate| original_files.contains(candidate))
        .map(Path::to_path_buf)
        .collect();
    if !matches.is_empty() {
        return matches;
    }
    if in_dir.exists() {
        return vec![in_dir];
    }
    if p.exists() {
        return vec![p.to_path_buf()];
    }
    vec![p.to_path_buf()]
}

/// Attempt to canonicalize a path, falling back to the original if it fails.
///
/// This is useful for comparing paths that may have been deleted or renamed,
/// where canonicalization would fail but we still want to match them.
pub(crate) fn try_canonicalize(path: &PathBuf) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(err) => {
            warn!("failed to canonicalize file: {} {err}", display_path(path));
            path.clone()
        }
    }
}

impl Step {
    /// Parse check_list_files output to extract files needing fixes.
    ///
    /// The command outputs one file path per line. This function:
    /// 1. Parses each line as a file path, resolving relative to `dir` (or `self.dir` if not templated) if needed
    /// 2. Matches each resulting path against the job's files as written,
    ///    falling back to canonical-path comparison only where that finds no
    ///    match (see [`match_listed_files`]), so naming a symlink doesn't also
    ///    select its target
    ///
    /// # Arguments
    ///
    /// * `original_files` - The files that were passed to the check command
    /// * `stdout` - The stdout output from check_list_files
    /// * `dir` - The step's rendered `dir`. Tools that run there usually print
    ///   paths relative to it, but some print paths relative to the root, so a
    ///   relative path matches a job file under either reading.
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// * Files from original_files that were listed in the output
    /// * Extra files that were listed but not in original_files (warnings)
    pub(crate) fn filter_files_from_check_list(
        &self,
        original_files: &[PathBuf],
        stdout: &str,
        dir: Option<&str>,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let dir = dir.or_else(|| {
            if self.dir_is_templated() {
                None
            } else {
                self.dir.as_deref()
            }
        });
        let original_set: HashSet<&Path> = original_files.iter().map(|p| p.as_path()).collect();
        // Each line's readings are matched as written before any canonical
        // fallback (see `match_listed_files`), so naming a symlink selects
        // only that symlink even when its target is also a job file.
        let readings: Vec<Vec<PathBuf>> = stdout
            .lines()
            .filter(|line| !line.is_empty())
            .map(|p| resolve_path(p, dir, &original_set))
            .collect();
        match_listed_files(original_files, &readings)
    }

    /// Parse unified diff output to extract files needing fixes.
    ///
    /// Extracts file paths from `---` and `+++` lines in unified diff format.
    /// Handles both standard diff output and git-style diffs with `a/` and `b/` prefixes.
    ///
    /// Also handles timestamp suffixes (e.g., `--- file.py\t2025-01-01 12:00:00`).
    ///
    /// # Arguments
    ///
    /// * `original_files` - The files that were passed to the check command
    /// * `stdout` - The stdout output containing unified diff
    /// * `dir` - The step's rendered `dir`, where the patch would apply, which
    ///   decides how a patch that only creates or deletes files reads its paths
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// * Files from original_files that appear in the diff
    /// * Extra files in the diff but not in original_files (warnings)
    pub(crate) fn filter_files_from_check_diff(
        &self,
        original_files: &[PathBuf],
        stdout: &str,
        dir: Option<&str>,
    ) -> DiffFiles {
        let stdout = normalize_diff_paths(stdout);
        // Match the paths that patch application will use, including absolute headers
        // made relative to the command's working directory.
        let base = PathBuf::from(dir.unwrap_or("."));
        let base = base.canonicalize().unwrap_or(base);
        let stdout = super::diff::relativize_diff_paths(&stdout, &base);

        // Parse unified diff format to extract file names from --- and +++ lines
        let mut listed: IndexSet<String> = IndexSet::new();
        let mut created: IndexSet<String> = IndexSet::new();

        // Only header pairs outside hunk bodies are read: a hunk can hold lines
        // that look like headers, such as a removed `-- x`.
        let strip_prefixes = strips_git_prefixes(&stdout, &base);
        for (old, new) in header_pairs(&stdout) {
            let creates_file = header_path(old) == "/dev/null";
            for (side, prefix, is_new) in [(old, "a/", false), (new, "b/", true)] {
                let path = header_path(side);
                // A created or deleted file has `/dev/null` on the other side.
                if path == "/dev/null" {
                    continue;
                }
                let path = if strip_prefixes {
                    path.strip_prefix(prefix).unwrap_or(&path)
                } else {
                    &path
                };
                if creates_file && is_new {
                    created.insert(path.to_string());
                } else {
                    listed.insert(path.to_string());
                }
            }
        }
        // The command ran in the step's `dir`, so a relative path usually names
        // a file there, but some tools print paths relative to the root. Each
        // reading that names a job file matches: an extra file only means the
        // fixer or staging touches one it didn't need to, while picking one
        // reading could lose the fix to the other.
        let readings: Vec<Vec<PathBuf>> = listed
            .into_iter()
            .map(|path| {
                let path = Path::new(&path);
                let in_dir = dir
                    .filter(|_| path.is_relative())
                    .map(|dir| Path::new(dir).join(path))
                    .filter(|in_dir| in_dir.symlink_metadata().is_ok());
                in_dir.into_iter().chain([path.to_path_buf()]).collect()
            })
            .collect();
        let (files, extras) = match_listed_files(original_files, &readings);
        DiffFiles {
            files,
            created: created.into_iter().map(PathBuf::from).collect(),
            extras,
        }
    }
}

/// The files in `original_files` that the `listed` paths name, and the
/// canonicalized listed paths that name none of them. Each listed path comes
/// as its possible readings, the one in the step's `dir` first when there are
/// two; every reading that names a file matches it, and the path is an extra
/// only if none does.
///
/// Paths are matched as written first, which is how tools usually print the
/// paths they were given. Only a listed path that matches no file that way is
/// canonicalized and compared with every file canonicalized. Canonicalizing
/// every file up front stats each component of each path, which takes tens of
/// milliseconds for a job of a few thousand files.
fn match_listed_files(
    original_files: &[PathBuf],
    listed: &[Vec<PathBuf>],
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let originals: HashSet<&Path> = original_files.iter().map(PathBuf::as_path).collect();
    let mut matched: HashSet<&Path> = HashSet::new();
    let mut unmatched: Vec<&[PathBuf]> = Vec::new();
    // Readings in `dir` that didn't match as written, although the root reading
    // did: `dir` may be a symlink, so its reading can name a job file only once
    // canonicalized. A root reading that doesn't match is just not a job file.
    let mut unmatched_in_dir: Vec<&PathBuf> = Vec::new();
    for readings in listed {
        let direct: Vec<&Path> = readings
            .iter()
            .filter_map(|reading| originals.get(reading.as_path()).copied())
            .collect();
        if direct.is_empty() {
            unmatched.push(readings);
        } else {
            matched.extend(direct);
            if let [in_dir, _] = readings.as_slice()
                && !originals.contains(in_dir.as_path())
            {
                unmatched_in_dir.push(in_dir);
            }
        }
    }
    if !unmatched_in_dir.is_empty() {
        // Resolve just these paths, and look up the result as written, either
        // absolute or relative to where hk runs, rather than canonicalizing
        // every job file.
        let cwd = std::env::current_dir()
            .ok()
            .and_then(|cwd| cwd.canonicalize().ok());
        for in_dir in unmatched_in_dir {
            // With the whole path resolved, and with only its directory
            // resolved: a job file can itself be a symlink, which the job names
            // by its own path rather than its target's.
            let resolved = [
                in_dir.canonicalize().ok(),
                in_dir
                    .parent()
                    .and_then(|dir| dir.canonicalize().ok())
                    .zip(in_dir.file_name())
                    .map(|(dir, name)| dir.join(name)),
            ];
            for path in resolved.iter().flatten() {
                let relative = cwd.as_deref().and_then(|cwd| path.strip_prefix(cwd).ok());
                for candidate in [Some(path.as_path()), relative].into_iter().flatten() {
                    if let Some(&original) = originals.get(candidate) {
                        matched.insert(original);
                    }
                }
            }
        }
    }
    let mut extras: IndexSet<PathBuf> = IndexSet::new();
    if !unmatched.is_empty() {
        let mut by_canonical: HashMap<PathBuf, Vec<&Path>> = HashMap::new();
        for file in original_files {
            by_canonical
                .entry(try_canonicalize(file))
                .or_default()
                .push(file);
        }
        for readings in unmatched {
            let canonical: Vec<PathBuf> = readings.iter().map(try_canonicalize).collect();
            let hits: Vec<&Path> = canonical
                .iter()
                .filter_map(|path| by_canonical.get(path))
                .flatten()
                .copied()
                .collect();
            if hits.is_empty() {
                extras.extend(canonical.into_iter().next());
            } else {
                matched.extend(hits);
            }
        }
    }
    let files: IndexSet<PathBuf> = original_files
        .iter()
        .filter(|f| matched.contains(f.as_path()))
        .cloned()
        .collect();
    (files.into_iter().collect(), extras.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_list_matches_job_files_relative_to_dir_or_root() {
        // Tests run from the crate root, so `Cargo.toml` names a file relative
        // to the root. The step's `dir` holds a file of the same name.
        let dir = tempfile::tempdir().unwrap();
        let in_dir = dir.path().join("Cargo.toml");
        std::fs::write(&in_dir, "").unwrap();
        let root = PathBuf::from("Cargo.toml");
        let dir_str = dir.path().to_str();
        let step = Step::default();

        let (files, extras) =
            step.filter_files_from_check_list(std::slice::from_ref(&root), "Cargo.toml\n", dir_str);
        assert_eq!(files, vec![root.clone()]);
        assert!(extras.is_empty());

        let (files, _) = step.filter_files_from_check_list(
            std::slice::from_ref(&in_dir),
            "Cargo.toml\n",
            dir_str,
        );
        assert_eq!(files, vec![in_dir.clone()]);

        let (files, extras) = step.filter_files_from_check_list(
            std::slice::from_ref(&in_dir),
            "missing.py\n",
            dir_str,
        );
        assert!(files.is_empty());
        assert_eq!(extras, vec![PathBuf::from("missing.py")]);
    }

    #[test]
    fn check_diff_matches_job_files_as_written_or_canonicalized() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        let c = dir.path().join("c.txt");
        for f in [&a, &b, &c] {
            std::fs::write(f, "x").unwrap();
        }
        let dot_b = dir.path().join(".").join("b.txt");
        let missing = dir.path().join("missing.txt");
        let diff = diff_naming(&[&a, &dot_b, &missing]);
        let step = Step::default();

        let DiffFiles {
            files,
            created,
            extras,
        } = step.filter_files_from_check_diff(&[c.clone(), b.clone(), a.clone()], &diff, None);
        // Job order is kept; `./b.txt` names `b.txt` once canonicalized.
        assert_eq!(files, vec![b, a]);
        assert!(created.is_empty());
        assert_eq!(extras.len(), 1);
        assert!(extras[0].ends_with("missing.txt"), "{extras:?}");
    }

    #[test]
    fn check_diff_reads_a_quoted_path_with_a_tab() {
        let step = Step::default();
        let tabbed = PathBuf::from("foo\tbar.txt");
        let files = vec![PathBuf::from("foo"), tabbed.clone()];
        let stdout = "--- \"a/foo\\tbar.txt\"\n+++ \"b/foo\\tbar.txt\"\n@@ -1 +1 @@\n-x  \n+x\n";
        let DiffFiles {
            files: matched,
            created,
            extras,
        } = step.filter_files_from_check_diff(&files, stdout, None);
        assert_eq!(matched, vec![tabbed]);
        assert!(created.is_empty());
        assert!(extras.is_empty());
    }

    #[test]
    fn check_diff_reads_a_deletion_only_patch_as_applying_would() {
        // `a/old.txt` names `old.txt` when that exists in the step's dir and
        // `a/old.txt` doesn't, as when the patch is applied.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.txt"), "x\n").unwrap();
        let step = Step::default();
        let job_file = dir.path().join("old.txt");
        let diff = "--- a/old.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        let DiffFiles { files, extras, .. } = step.filter_files_from_check_diff(
            std::slice::from_ref(&job_file),
            diff,
            dir.path().to_str(),
        );
        assert_eq!(files, vec![job_file]);
        assert!(extras.is_empty());
    }

    #[test]
    fn check_diff_paths_are_read_in_the_steps_dir() {
        // The command ran in `pkg`, so `a/x` names `pkg/x`. A root `x` is also a
        // job file and might be what a tool printing root paths meant, so both
        // match: running the fixer on both can't lose either fix.
        let root = tempfile::tempdir().unwrap();
        let pkg = root.path().join("pkg");
        std::fs::create_dir(&pkg).unwrap();
        std::fs::write(pkg.join("x"), "x\n").unwrap();
        let in_pkg = pkg.join("x");
        // Job files are relative to the root, where hk runs.
        let at_root = PathBuf::from("x");
        let step = Step::default();
        let diff = "--- a/x\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        let files = step.filter_files_from_check_diff(
            &[at_root.clone(), in_pkg.clone()],
            diff,
            pkg.to_str(),
        );
        assert_eq!(files.files, vec![at_root, in_pkg.clone()]);

        // With no root `x` in the job, only `pkg/x` matches, and no extra is
        // reported for the other reading.
        let DiffFiles { files, extras, .. } =
            step.filter_files_from_check_diff(std::slice::from_ref(&in_pkg), diff, pkg.to_str());
        assert_eq!(files, vec![in_pkg]);
        assert!(extras.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn check_diff_paths_in_a_symlinked_dir_match_the_real_file() {
        // The step runs in `alias`, a link to `pkg`. `a/x` reads as a root `x`,
        // which is a job file, and as `alias/x`, which is `pkg/x`.
        let root = tempfile::tempdir().unwrap();
        // Job files name resolved paths, as they do relative to the root.
        let pkg = root.path().canonicalize().unwrap().join("pkg");
        std::fs::create_dir(&pkg).unwrap();
        std::fs::write(pkg.join("x"), "x\n").unwrap();
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(&pkg, &alias).unwrap();
        let in_pkg = pkg.join("x");
        let at_root = PathBuf::from("x");
        let step = Step::default();
        let diff = "--- a/x\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        let files = step.filter_files_from_check_diff(
            &[at_root.clone(), in_pkg.clone()],
            diff,
            alias.to_str(),
        );
        assert_eq!(files.files, vec![at_root, in_pkg]);
    }

    #[cfg(unix)]
    #[test]
    fn check_diff_paths_in_a_symlinked_dir_match_a_symlinked_job_file() {
        // `alias` links to `pkg`, and the job file `pkg/link` is itself a link.
        let root = tempfile::tempdir().unwrap();
        let real = root.path().canonicalize().unwrap();
        let pkg = real.join("pkg");
        std::fs::create_dir(&pkg).unwrap();
        std::fs::write(pkg.join("target"), "x\n").unwrap();
        std::os::unix::fs::symlink(pkg.join("target"), pkg.join("link")).unwrap();
        let alias = real.join("alias");
        std::os::unix::fs::symlink(&pkg, &alias).unwrap();
        let link = pkg.join("link");
        let at_root = PathBuf::from("link");
        let step = Step::default();
        let diff = "--- a/link\n+++ b/link\n@@ -1 +1 @@\n-x\n+y\n";
        let files = step.filter_files_from_check_diff(
            &[at_root.clone(), link.clone()],
            diff,
            alias.to_str(),
        );
        assert_eq!(files.files, vec![at_root, link]);
    }

    fn diff_naming(paths: &[&Path]) -> String {
        paths
            .iter()
            .map(|p| format!("--- {p}\n+++ {p}\n@@ -1 +1 @@\n-x\n+y\n", p = p.display()))
            .collect()
    }

    #[cfg(unix)]
    #[test]
    fn check_diff_matches_a_symlinked_path_once_canonicalized() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        std::fs::write(&target, "x").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let step = Step::default();

        // The diff names the job's file only through a symlink to it.
        let DiffFiles { files, extras, .. } = step.filter_files_from_check_diff(
            std::slice::from_ref(&target),
            &diff_naming(&[&link]),
            None,
        );
        assert_eq!(files, vec![target]);
        assert!(extras.is_empty(), "{extras:?}");
    }

    #[cfg(unix)]
    #[test]
    fn check_diff_selects_only_the_alias_it_names_as_written() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        std::fs::write(&target, "x").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let step = Step::default();
        let job_files = [link.clone(), target.clone()];

        // With a symlink and its target both in the job (only possible with
        // `allow_symlinks`), a diff naming one of them as written selects
        // only that one.
        let DiffFiles { files, extras, .. } =
            step.filter_files_from_check_diff(&job_files, &diff_naming(&[&target]), None);
        assert_eq!(files, vec![target.clone()]);
        assert!(extras.is_empty(), "{extras:?}");
        let files = step.filter_files_from_check_diff(&job_files, &diff_naming(&[&link]), None);
        assert_eq!(files.files, vec![link]);
    }

    #[test]
    fn check_diff_distinguishes_created_files_from_unexpected_files() {
        let diff = "--- /dev/null\n+++ b/go.sum\n@@ -0,0 +1 @@\n+sum\n--- a/go.mod\n+++ b/go.mod\n@@ -1 +1 @@\n-old\n+new\n--- a/other.txt\n+++ b/other.txt\n@@ -1 +1 @@\n-old\n+new\n";
        let parsed =
            Step::default().filter_files_from_check_diff(&[PathBuf::from("go.mod")], diff, None);
        assert_eq!(parsed.files, vec![PathBuf::from("go.mod")]);
        assert_eq!(parsed.created, vec![PathBuf::from("go.sum")]);
        assert_eq!(parsed.extras, vec![PathBuf::from("other.txt")]);
    }

    #[test]
    fn check_diff_preserves_literal_prefix_in_mixed_patch() {
        let diff = "--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1 @@\n+new\n--- sub/existing.txt\n+++ sub/existing.txt\n@@ -1 +1 @@\n-old\n+updated\n";
        let parsed = Step::default().filter_files_from_check_diff(
            &[PathBuf::from("sub/existing.txt")],
            diff,
            None,
        );
        assert_eq!(parsed.files, vec![PathBuf::from("sub/existing.txt")]);
        assert_eq!(parsed.created, vec![PathBuf::from("b/new.txt")]);
        assert!(parsed.extras.is_empty());
    }

    #[test]
    fn test_filter_files_from_check_list_subproject() {
        let mut step = Step::default();
        step.dir = Some("frontend".to_string());
        let original_files = vec![
            PathBuf::from("frontend/src/App.tsx"),
            PathBuf::from("frontend/src/index.ts"),
        ];
        let stdout = "src/App.tsx\n";
        let (files, extras) = step.filter_files_from_check_list(&original_files, stdout, None);
        assert_eq!(files, vec![PathBuf::from("frontend/src/App.tsx")]);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }

    #[test]
    fn test_filter_files_from_check_list_subproject_with_dot_slash() {
        let mut step = Step::default();
        step.dir = Some("frontend".to_string());
        let original_files = vec![
            PathBuf::from("frontend/src/App.tsx"),
            PathBuf::from("frontend/src/index.ts"),
        ];
        let stdout = "./src/App.tsx\n";
        let (files, extras) = step.filter_files_from_check_list(&original_files, stdout, None);
        assert_eq!(files, vec![PathBuf::from("frontend/src/App.tsx")]);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }

    #[test]
    fn test_filter_files_from_check_list_root_file_does_not_hide_subproject_file() {
        let mut step = Step::default();
        step.dir = Some("frontend".to_string());
        let original_files = vec![PathBuf::from("frontend/src/App.ts")];
        let stdout = "src/App.ts\n";
        let (files, extras) = step.filter_files_from_check_list(&original_files, stdout, None);
        assert_eq!(files, vec![PathBuf::from("frontend/src/App.ts")]);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }

    #[test]
    fn test_filter_files_from_check_list_ambiguous_path_matches_both_job_files() {
        // The job holds both a root file and a subproject file that share the
        // same relative path. A report of the ambiguous path must match both,
        // not just the one under `dir`, or the root file is silently dropped.
        let mut step = Step::default();
        step.dir = Some("frontend".to_string());
        let original_files = vec![
            PathBuf::from("src/App.ts"),
            PathBuf::from("frontend/src/App.ts"),
        ];
        let stdout = "src/App.ts\n";
        let (mut files, extras) = step.filter_files_from_check_list(&original_files, stdout, None);
        files.sort();
        assert_eq!(
            files,
            vec![
                PathBuf::from("frontend/src/App.ts"),
                PathBuf::from("src/App.ts"),
            ]
        );
        assert_eq!(extras, Vec::<PathBuf>::new());
    }

    #[cfg(unix)]
    #[test]
    fn test_filter_files_from_check_list_selects_only_the_alias_it_names_as_written() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        std::fs::write(&target, "x").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let step = Step::default();
        let job_files = [link.clone(), target.clone()];

        // With a symlink and its target both in the job (only possible with
        // `allow_symlinks`), a report naming one of them as written selects
        // only that one, not both via canonical-path matching.
        let stdout = format!("{}\n", link.display());
        let (files, extras) = step.filter_files_from_check_list(&job_files, &stdout, None);
        assert_eq!(files, vec![link]);
        assert!(extras.is_empty(), "{extras:?}");
    }

    #[test]
    fn test_filter_files_from_check_list_templated_dir() {
        let mut step = Step::default();
        step.dir = Some("{{workspace}}".to_string());
        let original_files = vec![PathBuf::from("pkgs/api/main.go")];
        let stdout = "main.go\n";
        let (files, extras) =
            step.filter_files_from_check_list(&original_files, stdout, Some("pkgs/api"));
        assert_eq!(files, vec![PathBuf::from("pkgs/api/main.go")]);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }

    #[test]
    fn test_filter_files_from_check_list_preserves_filename_whitespace() {
        let step = Step::default();
        let original_files = vec![
            PathBuf::from(" file_with_leading_space.txt"),
            PathBuf::from("file_with_trailing_space.txt "),
        ];
        let stdout = " file_with_leading_space.txt\nfile_with_trailing_space.txt \n\n";
        let (files, extras) = step.filter_files_from_check_list(&original_files, stdout, None);
        assert_eq!(files, original_files);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }

    #[test]
    fn test_filter_files_from_check_list_subproject_preserves_filename_whitespace() {
        let mut step = Step::default();
        step.dir = Some("frontend".to_string());
        let original_files = vec![
            PathBuf::from("frontend/ file_with_leading_space.txt"),
            PathBuf::from("frontend/file_with_trailing_space.txt "),
        ];
        let stdout = " file_with_leading_space.txt\nfile_with_trailing_space.txt \n";
        let (files, extras) = step.filter_files_from_check_list(&original_files, stdout, None);
        assert_eq!(files, original_files);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }
}
