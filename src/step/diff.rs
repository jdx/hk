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
    // `ParseOptions::unidiff` doesn't read git's extended headers, so a mode
    // change would be dropped silently. Leave such patches to the fixer.
    if diff.lines().any(|line| {
        [
            "old mode ",
            "new mode ",
            "new file mode ",
            "deleted file mode ",
        ]
        .iter()
        .any(|header| line.starts_with(header))
    }) {
        return Err("patches that change file modes are not supported".to_string());
    }
    // Each touched file's contents after the patches so far, or `None` once
    // deleted. A file can appear in more than one file patch.
    let mut files: IndexMap<PathBuf, Option<String>> = IndexMap::new();
    // Each touched file's contents and permissions before the patch, or
    // `None` if it didn't exist, to restore if a write fails.
    type Original = (Option<String>, Option<std::fs::Permissions>);
    let mut originals: IndexMap<PathBuf, Original> = IndexMap::new();
    for file_patch in PatchSet::parse(diff, ParseOptions::unidiff()) {
        let file_patch = file_patch.map_err(|err| err.to_string())?;
        let PatchKind::Text(patch) = file_patch.patch() else {
            return Err("binary patches are not supported".to_string());
        };
        let known = |path: &PathBuf| files.contains_key(path) || base.join(path).exists();
        let (path, create, delete) = match file_patch.operation().strip_prefix(strip) {
            // Some tools diff against a temporary file or a label, so use the
            // side that names a file under `base`.
            FileOperation::Modify { original, modified } => {
                match checked_path(&original).ok().filter(known) {
                    Some(original) => (original, false, false),
                    None => (checked_path(&modified)?, false, false),
                }
            }
            FileOperation::Create(path) => (checked_path(&path)?, true, false),
            FileOperation::Delete(path) => (checked_path(&path)?, false, true),
            _ => return Err("renames and copies are not supported".to_string()),
        };
        let display = path.display().to_string();
        refuse_symlinks(base, &path)?;
        refuse_hard_links(&base.join(&path))?;
        let current = match files.get(&path) {
            Some(current) => current.clone(),
            None => {
                let target = base.join(&path);
                let current = read_existing(&target)?;
                let permissions = std::fs::metadata(&target).map(|m| m.permissions());
                originals.insert(path.clone(), (current.clone(), permissions.ok()));
                current
            }
        };
        if create && current.is_some() {
            return Err(format!("{display}: the patch creates a file that exists"));
        }
        if !create && current.is_none() {
            return Err(format!("{display}: no such file"));
        }
        let patched = diffy::apply(current.as_deref().unwrap_or_default(), patch)
            .map_err(|err| format!("{display}: {err}"))?;
        files.insert(path, (!delete).then_some(patched));
    }
    if files.is_empty() {
        return Err("no file patches found".to_string());
    }
    let mut written: Vec<&PathBuf> = Vec::with_capacity(files.len());
    for (path, contents) in &files {
        // Listed before writing: a write that fails after truncating the file
        // is rolled back too.
        written.push(path);
        if let Err(err) = write_contents(&base.join(path), contents.as_deref()) {
            // Put back what was already written, so the fixer starts from the
            // files as they were.
            for done in written {
                let target = base.join(done);
                let (contents, permissions): &Original = &originals[done];
                let _ = write_contents(&target, contents.as_deref());
                if let (Some(_), Some(permissions)) = (contents, permissions) {
                    let _ = std::fs::set_permissions(&target, permissions.clone());
                }
            }
            return Err(format!("{}: {err}", path.display()));
        }
    }
    Ok(files.len())
}

/// Write `contents` to `path`, creating its directory, or remove it for `None`.
///
/// An existing file is written in place, keeping its owner, permissions, ACLs,
/// and extended attributes; a read-only one fails the write, and hk runs the
/// fixer. A new file gets the usual permissions for the user's umask.
fn write_contents(path: &Path, contents: Option<&str>) -> std::io::Result<()> {
    match contents {
        Some(contents) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, contents)
        }
        None => match std::fs::remove_file(path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        },
    }
}

/// Refuse a file with other hard links, which may be outside the working
/// directory: writing it in place would change them too.
#[cfg(unix)]
fn refuse_hard_links(path: &Path) -> std::result::Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && meta.nlink() > 1 => {
            Err(format!("{}: has other hard links", path.display()))
        }
        _ => Ok(()),
    }
}

#[cfg(not(unix))]
fn refuse_hard_links(_path: &Path) -> std::result::Result<(), String> {
    Ok(())
}

/// Refuse `path` if it or any directory leading to it under `base` is a
/// symlink, which a write would follow out of the working directory.
fn refuse_symlinks(base: &Path, path: &Path) -> std::result::Result<(), String> {
    let mut prefix = PathBuf::new();
    for component in path.components() {
        prefix.push(component);
        match std::fs::symlink_metadata(base.join(&prefix)) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!("{}: is a symlink", prefix.display()));
            }
            Ok(_) => {}
            // Nothing below a missing directory exists yet.
            Err(_) => break,
        }
    }
    Ok(())
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

    #[cfg(unix)]
    #[test]
    fn refuses_writes_through_a_symlinked_directory() {
        let outside = dir_with(&[("file.txt", "x\n")]);
        let dir = dir_with(&[]);
        std::os::unix::fs::symlink(outside.path(), dir.path().join("linked")).unwrap();
        let diff = "--- linked/file.txt\n+++ linked/file.txt\n@@ -1 +1 @@\n-x\n+y\n";
        assert!(
            apply_patch(diff, 0, dir.path())
                .unwrap_err()
                .contains("symlink")
        );
        assert_eq!(read(&outside, "file.txt"), "x\n");
    }

    #[test]
    fn refuses_to_create_a_file_that_exists() {
        let dir = dir_with(&[("a.txt", "keep\n")]);
        let diff = "--- /dev/null\n+++ a.txt\n@@ -0,0 +1 @@\n+added\n";
        assert!(
            apply_patch(diff, 0, dir.path())
                .unwrap_err()
                .contains("exists")
        );
        assert_eq!(read(&dir, "a.txt"), "keep\n");
    }

    #[test]
    fn refuses_mode_changes() {
        let dir = dir_with(&[("a.sh", "x\n")]);
        let diff = "diff --git a/a.sh b/a.sh\nold mode 100644\nnew mode 100755\n\
                    --- a/a.sh\n+++ b/a.sh\n@@ -1 +1 @@\n-x\n+y\n";
        assert!(
            apply_patch(diff, 1, dir.path())
                .unwrap_err()
                .contains("mode")
        );
        assert_eq!(read(&dir, "a.sh"), "x\n");
    }

    #[test]
    fn restores_written_files_when_a_later_write_fails() {
        // `blocker` is a file, so `blocker/new.txt` can't be created.
        let dir = dir_with(&[("a.txt", "x\n"), ("blocker", "")]);
        let diff = "--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-x\n+y\n\
                    --- /dev/null\n+++ blocker/new.txt\n@@ -0,0 +1 @@\n+hi\n";
        assert!(apply_patch(diff, 0, dir.path()).is_err());
        assert_eq!(read(&dir, "a.txt"), "x\n");
    }

    #[cfg(unix)]
    #[test]
    fn restores_a_deleted_file_with_its_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir_with(&[("run.sh", "x\n"), ("blocker", "")]);
        fs::set_permissions(dir.path().join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
        let diff = "--- run.sh\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n\
                    --- /dev/null\n+++ blocker/new.txt\n@@ -0,0 +1 @@\n+hi\n";
        assert!(apply_patch(diff, 0, dir.path()).is_err());
        assert_eq!(read(&dir, "run.sh"), "x\n");
        let mode = fs::metadata(dir.path().join("run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }

    #[test]
    fn falls_back_to_the_modified_side_when_the_original_is_unusable() {
        let dir = dir_with(&[("a.txt", "x\n")]);
        let diff = "--- /tmp/original-a.txt\n+++ a.txt\n@@ -1 +1 @@\n-x\n+y\n";
        assert_eq!(apply_patch(diff, 0, dir.path()), Ok(1));
        assert_eq!(read(&dir, "a.txt"), "y\n");
    }

    #[cfg(unix)]
    #[test]
    fn refuses_files_with_other_hard_links() {
        let outside = dir_with(&[("file.txt", "x\n")]);
        let dir = dir_with(&[]);
        fs::hard_link(outside.path().join("file.txt"), dir.path().join("file.txt")).unwrap();
        let diff = "--- file.txt\n+++ file.txt\n@@ -1 +1 @@\n-x\n+y\n";
        assert!(
            apply_patch(diff, 0, dir.path())
                .unwrap_err()
                .contains("hard links")
        );
        assert_eq!(read(&outside, "file.txt"), "x\n");
    }

    #[cfg(unix)]
    #[test]
    fn keeps_permissions_and_gives_new_files_the_usual_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir_with(&[("run.sh", "x\n")]);
        fs::set_permissions(dir.path().join("run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
        let diff = "--- run.sh\n+++ run.sh\n@@ -1 +1 @@\n-x\n+y\n\
                    --- /dev/null\n+++ new.txt\n@@ -0,0 +1 @@\n+hi\n";
        assert_eq!(apply_patch(diff, 0, dir.path()), Ok(2));
        let mode = |name: &str| {
            fs::metadata(dir.path().join(name))
                .unwrap()
                .permissions()
                .mode()
        };
        assert_eq!(mode("run.sh") & 0o777, 0o755);
        // Created like any other file: readable by others unless the umask
        // says otherwise, not the owner-only mode of a temporary file.
        let umask_allows_other_read = {
            let probe = dir.path().join("probe");
            fs::write(&probe, "").unwrap();
            mode("probe") & 0o044
        };
        assert_eq!(mode("new.txt") & 0o044, umask_allows_other_read);
    }

    #[cfg(unix)]
    #[test]
    fn leaves_read_only_files_to_the_fixer() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir_with(&[("a.txt", "x\n")]);
        fs::set_permissions(dir.path().join("a.txt"), fs::Permissions::from_mode(0o444)).unwrap();
        let diff = "--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-x\n+y\n";
        // Root can write read-only files, so only check the outcome matches.
        let result = apply_patch(diff, 0, dir.path());
        assert_eq!(result.is_ok(), read(&dir, "a.txt") == "y\n");
        assert_eq!(
            fs::metadata(dir.path().join("a.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o444
        );
    }

    #[test]
    fn output_without_a_patch_is_an_error() {
        let dir = dir_with(&[]);
        assert!(apply_patch("Issues were detected.\n", 0, dir.path()).is_err());
    }
}
