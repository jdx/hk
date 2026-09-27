//! Applying unified diffs directly to files.
//!
//! When a step has `check_diff` configured, instead of running the fixer command,
//! hk can apply the diff output directly using `git apply`. This is often faster
//! than running the fixer, especially for tools that are slow to start.

use crate::Result;
use diffy::patch_set::{FileOperation, ParseOptions, PatchKind, PatchSet};
use indexmap::IndexMap;
use std::path::{Path, PathBuf};

use super::types::Step;
use super::{diff_lines, normalize_diff_paths, split_line_ending, strips_git_prefixes};

/// Rewrite absolute paths in diff headers to be relative to `base`.
///
/// `git apply` rejects absolute paths outright -- `--unsafe-paths` does not
/// change that, and `-p<n>` would need a strip depth that varies per checkout --
/// so a tool reporting absolute paths cannot be applied without this. `go fix
/// -diff` is one such tool.
///
/// Paths outside `base` are left as they are; `git apply` will reject them and
/// the caller falls back to running the fixer.
fn relativize_diff_paths(diff: &str, base: &Path) -> String {
    let mut out = String::with_capacity(diff.len() + 1);
    // Keep each line's terminator: a changed line's `\r` must survive.
    for (line, in_hunk) in diff_lines(diff) {
        if in_hunk {
            out.push_str(line);
            continue;
        }
        let (content, ending) = split_line_ending(line);
        let rewritten = ["--- ", "+++ "].into_iter().find_map(|prefix| {
            let rest = content.strip_prefix(prefix)?;
            // Keep any tab-separated timestamp attached to the path.
            let (path, tail) = match rest.split_once('\t') {
                Some((p, t)) => (p, Some(t)),
                None => (rest, None),
            };
            let rel = Path::new(path).strip_prefix(base).ok()?.to_str()?;
            Some(match tail {
                Some(t) => format!("{prefix}{rel}\t{t}{ending}"),
                None => format!("{prefix}{rel}{ending}"),
            })
        });
        match rewritten {
            Some(rewritten) => out.push_str(&rewritten),
            None => out.push_str(line),
        }
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

impl Step {
    /// Apply a unified diff directly to files using `git apply`.
    ///
    /// This provides a fast path for fixing files when `check_diff` is configured.
    /// Instead of running the potentially slow fixer command, the diff output
    /// can be applied directly.
    ///
    /// Automatically detects whether the diff uses `a/` and `b/` prefixes (git-style)
    /// and sets the appropriate strip level (`-p1` or `-p0`).
    ///
    /// Also handles Go-style diffs where the `---` line has a `.orig` suffix
    /// (e.g., `--- file.go.orig` instead of `--- file.go`).
    ///
    /// # Arguments
    ///
    /// * `stdout` - The unified diff output from the check_diff command
    /// * `dir` - The job's rendered working directory, or `None` for the repo root
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Diff was applied successfully
    /// * `Ok(false)` - Diff application failed (caller should fall back to fixer)
    /// * `Err(_)` - Unexpected error
    pub(crate) fn apply_diff_output(&self, stdout: &str, dir: Option<&str>) -> Result<bool> {
        if stdout.trim().is_empty() {
            debug!("{}: no diff content to apply", self.name);
            return Ok(false);
        }
        let diff_content = normalize_diff_paths(stdout);

        // Resolve against wherever `git apply` will run, so absolute paths
        // reported by the check command become paths git will accept.
        let base = PathBuf::from(dir.unwrap_or("."));
        let base = base.canonicalize().unwrap_or(base);
        let diff_content = relativize_diff_paths(&diff_content, &base);

        // Git-style `a/` and `b/` prefixes need -p1, other paths -p0.
        let strip_level = if strips_git_prefixes(&diff_content, &base) {
            "-p1"
        } else {
            "-p0"
        };

        let strip = if strip_level == "-p1" { 1 } else { 0 };
        match apply_patch(&diff_content, strip, &base) {
            Ok(files) => {
                debug!("{}: applied diff to {} file(s)", self.name, files);
                Ok(true)
            }
            Err(reason) => {
                // Output that is no patch at all is how some commands hand a
                // file to the fixer, such as shellcheck's note that nothing is
                // auto-fixable. A patch that doesn't apply is a broken
                // `check_diff` that makes every fix run the tool twice, so say so.
                if looks_like_patch(&diff_content) {
                    warn!(
                        "{}: check_diff printed a patch that doesn't apply, so the fixer ran instead: {reason}",
                        self.name
                    );
                } else {
                    debug!("{}: check_diff output is not a patch: {reason}", self.name);
                }
                Ok(false)
            }
        }
    }
}

/// Apply every file patch in `diff` to the files under `base`, stripping
/// `strip` leading path components, and return how many files changed.
///
/// Every file's new contents are worked out before any is written, so a patch
/// that doesn't apply changes nothing, as with `git apply`. Hunks may apply at
/// an offset from the line they name, but their context must match exactly.
fn apply_patch(diff: &str, strip: usize, base: &Path) -> std::result::Result<usize, String> {
    // Each touched file's contents after the patches so far, or `None` once
    // deleted. A file can appear in more than one file patch.
    let mut files: IndexMap<PathBuf, Option<String>> = IndexMap::new();
    for file_patch in PatchSet::parse(diff, ParseOptions::unidiff()) {
        let file_patch = file_patch.map_err(|err| err.to_string())?;
        let PatchKind::Text(patch) = file_patch.patch() else {
            return Err("binary patches are not supported".to_string());
        };
        let (path, delete) = match file_patch.operation().strip_prefix(strip) {
            // Some tools diff against a temporary file or a label, so use the
            // side that names a file under `base`.
            FileOperation::Modify { original, modified } => {
                let original = checked_path(&original)?;
                if files.contains_key(&original) || base.join(&original).exists() {
                    (original, false)
                } else {
                    (checked_path(&modified)?, false)
                }
            }
            FileOperation::Create(path) => (checked_path(&path)?, false),
            FileOperation::Delete(path) => (checked_path(&path)?, true),
            _ => return Err("renames and copies are not supported".to_string()),
        };
        let current = match files.get(&path) {
            Some(current) => current.clone(),
            None => read_existing(&base.join(&path))?,
        };
        let display = path.display();
        let patched = diffy::apply(current.as_deref().unwrap_or_default(), patch)
            .map_err(|err| format!("{display}: {err}"))?;
        files.insert(path, (!delete).then_some(patched));
    }
    if files.is_empty() {
        return Err("no file patches found".to_string());
    }
    for (path, contents) in &files {
        let target = base.join(path);
        let written = match contents {
            Some(contents) => target
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&target, contents)),
            None => std::fs::remove_file(&target),
        };
        written.map_err(|err| format!("{}: {err}", path.display()))?;
    }
    Ok(files.len())
}

/// `path` if it stays under the directory the patch applies in.
fn checked_path(path: &str) -> std::result::Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if path.as_os_str().is_empty()
        || path.components().any(|c| {
            !matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err(format!(
            "{}: not a path under the working directory",
            path.display()
        ));
    }
    Ok(path)
}

/// The contents of `path`, or `None` if it doesn't exist.
fn read_existing(path: &Path) -> std::result::Result<Option<String>, String> {
    match std::fs::symlink_metadata(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("{}: {err}", path.display())),
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(format!("{}: is a symlink", path.display()));
        }
        Ok(_) => {}
    }
    std::fs::read_to_string(path)
        .map(Some)
        .map_err(|err| format!("{}: {err}", path.display()))
}

/// Whether `diff` contains a unified diff file header: a `--- ` line directly
/// followed by a `+++ ` line.
fn looks_like_patch(diff: &str) -> bool {
    let mut lines = diff.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("--- ") && lines.peek().is_some_and(|next| next.starts_with("+++ ")) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod looks_like_patch_tests {
    use super::looks_like_patch;

    #[test]
    fn finds_a_file_header() {
        assert!(looks_like_patch(
            "note\n--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-a\n+b\n"
        ));
    }

    #[test]
    fn ignores_output_without_one() {
        assert!(!looks_like_patch(
            "Issues were detected, but none were auto-fixable. Use another format to see them.\n"
        ));
        assert!(!looks_like_patch("Diff in a.lua:\n1 |-print \"foo\"\n"));
        assert!(!looks_like_patch("--- only a separator\ntext\n"));
    }
}

#[cfg(test)]
mod relativize_diff_paths_tests {
    use super::relativize_diff_paths;
    use std::path::Path;

    #[test]
    fn rewrites_absolute_paths_under_base() {
        let diff = "--- /w/svc/main.go\n+++ /w/svc/main.go\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/w/svc")),
            "--- main.go\n+++ main.go\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn leaves_paths_outside_base_alone() {
        let diff = "--- /elsewhere/main.go\n+++ /elsewhere/main.go\n";
        assert_eq!(relativize_diff_paths(diff, Path::new("/w/svc")), diff);
    }

    #[test]
    fn leaves_relative_paths_alone() {
        let diff = "--- main.go\n+++ main.go\n";
        assert_eq!(relativize_diff_paths(diff, Path::new("/w/svc")), diff);
    }

    #[test]
    fn preserves_a_tab_separated_timestamp() {
        let diff = "--- /w/svc/main.go\t2025-01-01 12:00:00\n+++ /w/svc/main.go\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/w/svc")),
            "--- main.go\t2025-01-01 12:00:00\n+++ main.go\n"
        );
    }

    #[test]
    fn keeps_carriage_returns() {
        let diff = "--- /repo/f.txt\n+++ /repo/f.txt\n@@ -1 +1 @@\n-one\r\n+one\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/repo")),
            "--- f.txt\n+++ f.txt\n@@ -1 +1 @@\n-one\r\n+one\n"
        );
    }

    #[test]
    fn does_not_touch_diff_body_lines() {
        let diff = "--- /w/svc/a.go\n+++ /w/svc/a.go\n@@ -1 +1 @@\n---- not a header\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/w/svc")),
            "--- a.go\n+++ a.go\n@@ -1 +1 @@\n---- not a header\n"
        );
    }
}

#[cfg(test)]
mod apply_patch_tests {
    use super::apply_patch;
    use std::fs;

    fn dir_with(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, contents) in files {
            fs::write(dir.path().join(name), contents).unwrap();
        }
        dir
    }

    fn read(dir: &tempfile::TempDir, name: &str) -> String {
        fs::read_to_string(dir.path().join(name)).unwrap()
    }

    #[test]
    fn applies_every_file_and_keeps_carriage_returns() {
        let dir = dir_with(&[("a.txt", "one  \r\ntwo\r\n"), ("b.txt", "x\n")]);
        let diff = "--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n-one  \r\n+one\r\n two\r\n\
                    --- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-x\n+y\n";
        assert_eq!(apply_patch(diff, 1, dir.path()), Ok(2));
        assert_eq!(read(&dir, "a.txt"), "one\r\ntwo\r\n");
        assert_eq!(read(&dir, "b.txt"), "y\n");
    }

    #[test]
    fn changes_nothing_when_any_hunk_fails() {
        let dir = dir_with(&[("a.txt", "x\n"), ("b.txt", "p\n")]);
        let diff = "--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-x\n+y\n\
                    --- b.txt\n+++ b.txt\n@@ -1 +1 @@\n-nope\n+q\n";
        assert!(
            apply_patch(diff, 0, dir.path())
                .unwrap_err()
                .contains("b.txt")
        );
        assert_eq!(read(&dir, "a.txt"), "x\n");
        assert_eq!(read(&dir, "b.txt"), "p\n");
    }

    #[test]
    fn uses_the_side_that_names_an_existing_file() {
        // buildifier's `-mode=diff` compares with a temporary file.
        let dir = dir_with(&[("BUILD", "a\n")]);
        let diff = "--- BUILD\n+++ tmp/buildifier-tmp-123\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(apply_patch(diff, 0, dir.path()), Ok(1));
        assert_eq!(read(&dir, "BUILD"), "b\n");
    }

    #[test]
    fn creates_and_deletes_files() {
        let dir = dir_with(&[("old.txt", "bye\n")]);
        let diff = "--- /dev/null\n+++ b/sub/new.txt\n@@ -0,0 +1 @@\n+hi\n\
                    --- a/old.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-bye\n";
        assert_eq!(apply_patch(diff, 1, dir.path()), Ok(2));
        assert_eq!(read(&dir, "sub/new.txt"), "hi\n");
        assert!(!dir.path().join("old.txt").exists());
    }

    #[test]
    fn refuses_paths_outside_the_directory() {
        let dir = dir_with(&[]);
        for path in ["../escape.txt", "/etc/escape.txt"] {
            let diff = format!("--- {path}\n+++ {path}\n@@ -0,0 +1 @@\n+x\n");
            assert!(apply_patch(&diff, 0, dir.path()).is_err(), "{path}");
        }
    }

    #[test]
    fn output_without_a_patch_is_an_error() {
        let dir = dir_with(&[]);
        assert!(apply_patch("Issues were detected.\n", 0, dir.path()).is_err());
    }
}
