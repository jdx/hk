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

/// Attempt to canonicalize a path, falling back to the original if it fails.
///
/// This is useful for comparing paths that may have been deleted or renamed,
/// where canonicalization would fail but we still want to match them.
pub(crate) fn try_canonicalize(path: &PathBuf) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(err) => {
            warn!("failed to canonicalize file: {} {err}", display_path(path));
            path.to_path_buf()
        }
    }
}

impl Step {
    /// Parse check_list_files output to extract files needing fixes.
    ///
    /// The command outputs one file path per line. This function:
    /// 1. Parses each line as a file path
    /// 2. Canonicalizes paths for comparison
    /// 3. Filters to only include files from the original input
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
        let originals: HashSet<PathBuf> = original_files.iter().map(try_canonicalize).collect();
        let mut listed: HashSet<PathBuf> = HashSet::new();
        let mut extras: IndexSet<PathBuf> = IndexSet::new();
        for line in stdout.lines() {
            let path = PathBuf::from(line);
            let in_dir = dir
                .filter(|_| path.is_relative())
                .map(|dir| Path::new(dir).join(&path))
                .filter(|path| path.symlink_metadata().is_ok());
            let mut candidates: Vec<PathBuf> = in_dir.iter().map(try_canonicalize).collect();
            candidates.push(if path.symlink_metadata().is_ok() {
                try_canonicalize(&path)
            } else {
                path.clone()
            });
            let matched: Vec<PathBuf> = candidates
                .iter()
                .filter(|path| originals.contains(*path))
                .cloned()
                .collect();
            if !matched.is_empty() {
                listed.extend(matched);
            } else {
                extras.extend(candidates.into_iter().next());
            }
        }
        let files: Vec<PathBuf> = original_files
            .iter()
            .filter(|f| listed.contains(&try_canonicalize(f)))
            .cloned()
            .collect();
        (files, extras.into_iter().collect())
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
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let stdout = normalize_diff_paths(stdout);

        // Parse unified diff format to extract file names from --- and +++ lines
        let mut listed: IndexSet<&str> = IndexSet::new();

        // Only header pairs outside hunk bodies are read: a hunk can hold lines
        // that look like headers, such as a removed `-- x`.
        let strip_prefixes = strips_git_prefixes(&stdout, Path::new(dir.unwrap_or(".")));
        for (old, new) in header_pairs(&stdout) {
            for (side, prefix) in [(old, "a/"), (new, "b/")] {
                let path = header_path(side);
                // A created or deleted file has `/dev/null` on the other side.
                if path == "/dev/null" {
                    continue;
                }
                let path = if strip_prefixes {
                    path.strip_prefix(prefix).unwrap_or(path)
                } else {
                    path
                };
                listed.insert(path);
            }
        }
        match_listed_files(original_files, listed.into_iter().map(Path::new))
    }
}

/// The files in `original_files` that the `listed` paths name, and the
/// canonicalized listed paths that name none of them.
///
/// Paths are matched as written first, which is how tools usually print the
/// paths they were given. Only a listed path that matches no file that way is
/// canonicalized and compared with every file canonicalized. Canonicalizing
/// every file up front stats each component of each path, which takes tens of
/// milliseconds for a job of a few thousand files.
fn match_listed_files<'a>(
    original_files: &[PathBuf],
    listed: impl IntoIterator<Item = &'a Path>,
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let originals: HashSet<&Path> = original_files.iter().map(PathBuf::as_path).collect();
    let mut matched: HashSet<&Path> = HashSet::new();
    let mut unmatched: Vec<&Path> = Vec::new();
    for path in listed {
        match originals.get(path) {
            Some(original) => {
                matched.insert(original);
            }
            None => unmatched.push(path),
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
        for path in unmatched {
            let canonical = try_canonicalize(&path.to_path_buf());
            match by_canonical.get(&canonical) {
                Some(files) => matched.extend(files.iter().copied()),
                None => {
                    extras.insert(canonical);
                }
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

        let (files, extras) =
            step.filter_files_from_check_diff(&[c.clone(), b.clone(), a.clone()], &diff, None);
        // Job order is kept; `./b.txt` names `b.txt` once canonicalized.
        assert_eq!(files, vec![b, a]);
        assert_eq!(extras.len(), 1);
        assert!(extras[0].ends_with("missing.txt"), "{extras:?}");
    }

    #[test]
    fn check_diff_reads_a_deletion_only_patch_as_applying_would() {
        // `a/old.txt` names `old.txt` when that exists in the step's dir and
        // `a/old.txt` doesn't, as when the patch is applied.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.txt"), "x\n").unwrap();
        let step = Step::default();
        let job_file = PathBuf::from("old.txt");
        let diff = "--- a/old.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        let (files, extras) = step.filter_files_from_check_diff(
            std::slice::from_ref(&job_file),
            diff,
            dir.path().to_str(),
        );
        assert_eq!(files, vec![job_file]);
        assert!(extras.is_empty());
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
        let (files, extras) = step.filter_files_from_check_diff(
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
        let (files, extras) =
            step.filter_files_from_check_diff(&job_files, &diff_naming(&[&target]), None);
        assert_eq!(files, vec![target.clone()]);
        assert!(extras.is_empty(), "{extras:?}");
        let (files, _) =
            step.filter_files_from_check_diff(&job_files, &diff_naming(&[&link]), None);
        assert_eq!(files, vec![link]);
    }
}
