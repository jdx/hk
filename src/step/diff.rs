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
    /// * `Ok(false)` - Diff application failed and changed nothing (caller should fall back to fixer)
    /// * `Err(_)` - Writing failed and some files couldn't be restored, so the
    ///   fixer must not run on them
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
            Err(PatchError::RollbackFailed(reason)) => eyre::bail!(
                "{}: applying the check_diff patch failed and some files could not be restored; check them before running hk again: {reason}",
                self.name
            ),
            Err(PatchError::Rejected(reason)) => {
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

/// Why a patch wasn't applied.
#[derive(Debug, PartialEq)]
enum PatchError {
    /// Nothing was changed; the fixer can run.
    Rejected(String),
    /// A write failed and at least one file couldn't be put back.
    RollbackFailed(String),
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatchError::Rejected(reason) | PatchError::RollbackFailed(reason) => {
                f.write_str(reason)
            }
        }
    }
}

impl From<String> for PatchError {
    fn from(reason: String) -> Self {
        PatchError::Rejected(reason)
    }
}

/// Apply every file patch in `diff` to the files under `base`, stripping
/// `strip` leading path components, and return how many files changed.
///
/// Every file's new contents are worked out before any is written, so a patch
/// that doesn't apply changes nothing, as with `git apply`. Hunks may apply at
/// an offset from the line they name, but their context must match exactly.
fn apply_patch(diff: &str, strip: usize, base: &Path) -> std::result::Result<usize, PatchError> {
    // `ParseOptions::unidiff` doesn't read git's extended headers, so a mode
    // change would be dropped silently, and a rename or copy with hunks would
    // patch the old path as if it were modified in place. Binary changes have
    // no hunks at all. Leave such patches to the fixer.
    if let Some(header) = diff.lines().find_map(|line| {
        [
            "old mode ",
            "new mode ",
            "new file mode ",
            "deleted file mode ",
            "rename from ",
            "rename to ",
            "copy from ",
            "copy to ",
            "Binary files ",
            "GIT binary patch",
        ]
        .into_iter()
        .find(|header| line.starts_with(header))
    }) {
        return Err(format!(
            "patches with `{}` headers are not supported",
            header.trim_end()
        )
        .into());
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
            return Err("binary patches are not supported".to_string().into());
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
            _ => return Err("renames and copies are not supported".to_string().into()),
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
            return Err(format!("{display}: the patch creates a file that exists").into());
        }
        if !create && current.is_none() {
            return Err(format!("{display}: no such file").into());
        }
        let patched = diffy::apply(current.as_deref().unwrap_or_default(), patch)
            .map_err(|err| format!("{display}: {err}"))?;
        files.insert(path, (!delete).then_some(patched));
    }
    if files.is_empty() {
        return Err("no file patches found".to_string().into());
    }
    let mut written: Vec<&PathBuf> = Vec::with_capacity(files.len());
    // A deleted file is moved aside, not removed, until every write has
    // succeeded, so a rollback moves the same file back with its owner, ACLs,
    // and extended attributes.
    let mut set_aside: Vec<(&PathBuf, PathBuf)> = Vec::new();
    for (path, contents) in &files {
        let target = base.join(path);
        let result = match contents {
            Some(contents) => {
                // Listed before writing: a write that fails after truncating
                // the file is rolled back too.
                written.push(path);
                write_contents(&target, Some(contents))
            }
            None => move_aside(&target).map(|aside| set_aside.push((path, aside))),
        };
        if let Err(err) = result {
            // Put back what was already written, so the fixer starts from the
            // files as they were.
            let mut message = format!("{}: {err}", path.display());
            let mut restored = true;
            for (done, aside) in &set_aside {
                if let Err(err) = std::fs::rename(aside, base.join(done)) {
                    message.push_str(&format!("; could not restore {}: {err}", done.display()));
                    restored = false;
                }
            }
            for done in written {
                let attempted = files[done].as_deref();
                if let Err(err) = roll_back(&base.join(done), &originals[done], attempted) {
                    message.push_str(&format!("; could not restore {}: {err}", done.display()));
                    restored = false;
                }
            }
            return Err(if restored {
                PatchError::Rejected(message)
            } else {
                PatchError::RollbackFailed(message)
            });
        }
    }
    for (path, aside) in &set_aside {
        if let Err(err) = std::fs::remove_file(aside) {
            warn!(
                "{}: deleted by a check_diff patch, but its copy {} could not be removed: {err}",
                path.display(),
                aside.display()
            );
        }
    }
    Ok(files.len())
}

/// Rename `path` to a hidden name beside it, for a deletion that can be
/// undone, and return the new name.
fn move_aside(path: &Path) -> std::io::Result<PathBuf> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let dir = path.parent().unwrap_or(Path::new(""));
    let mut n = 0;
    let aside = loop {
        let aside = dir.join(format!(".{name}.hk-deleted-{}-{n}", std::process::id()));
        if std::fs::symlink_metadata(&aside).is_err() {
            break aside;
        }
        n += 1;
    };
    std::fs::rename(path, &aside)?;
    Ok(aside)
}

/// Put `path` back as it was before hk tried to write `attempted` to it.
///
/// Only undoes what hk may have done: a file that still has its original
/// contents is left alone, and a file hk was creating is removed only if it
/// holds a prefix of what hk wrote, so a file another process put there stays.
fn roll_back(
    path: &Path,
    (contents, permissions): &(Option<String>, Option<std::fs::Permissions>),
    attempted: Option<&str>,
) -> std::io::Result<()> {
    // A file hk can't read can't be checked, so its rollback fails rather
    // than guessing that it is absent or unchanged.
    let current = match std::fs::read(path) {
        Ok(current) => Some(current),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };
    match contents {
        Some(original) => {
            if current.as_deref() == Some(original.as_bytes()) {
                return Ok(());
            }
            write_contents(path, Some(original))?;
            if let Some(permissions) = permissions {
                std::fs::set_permissions(path, permissions.clone())?;
            }
            Ok(())
        }
        None => match current {
            Some(current) if attempted.is_some_and(|a| a.as_bytes().starts_with(&current)) => {
                write_contents(path, None)
            }
            _ => Ok(()),
        },
    }
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

#[cfg(windows)]
fn refuse_hard_links(path: &Path) -> std::result::Result<(), String> {
    let links = std::fs::File::open(path)
        .and_then(|file| winapi_util::file::information(&file))
        .map(|info| info.number_of_links());
    match links {
        Ok(links) if links > 1 => Err(format!("{}: has other hard links", path.display())),
        _ => Ok(()),
    }
}

#[cfg(not(any(unix, windows)))]
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
                .to_string()
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
                .to_string()
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
                .to_string()
                .contains("exists")
        );
        assert_eq!(read(&dir, "a.txt"), "keep\n");
    }

    #[test]
    fn refuses_renames_copies_and_binary_changes() {
        let dir = dir_with(&[("old.txt", "x\n")]);
        for headers in [
            "rename from old.txt\nrename to new.txt\n",
            "copy from old.txt\ncopy to new.txt\n",
            "Binary files a/old.txt and b/old.txt differ\n",
        ] {
            let diff = format!(
                "diff --git a/old.txt b/new.txt\n{headers}--- a/old.txt\n+++ b/new.txt\n@@ -1 +1 @@\n-x\n+y\n"
            );
            assert!(apply_patch(&diff, 1, dir.path()).is_err(), "{headers}");
            assert_eq!(read(&dir, "old.txt"), "x\n");
            assert!(!dir.path().join("new.txt").exists());
        }
    }

    #[test]
    fn refuses_mode_changes() {
        let dir = dir_with(&[("a.sh", "x\n")]);
        let diff = "diff --git a/a.sh b/a.sh\nold mode 100644\nnew mode 100755\n\
                    --- a/a.sh\n+++ b/a.sh\n@@ -1 +1 @@\n-x\n+y\n";
        assert!(
            apply_patch(diff, 1, dir.path())
                .unwrap_err()
                .to_string()
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
    fn restores_a_deleted_file_as_the_same_file() {
        use std::os::unix::fs::MetadataExt;
        let dir = dir_with(&[("gone.txt", "x\n"), ("blocker", "")]);
        let inode = fs::metadata(dir.path().join("gone.txt")).unwrap().ino();
        let diff = "--- gone.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n\
                    --- /dev/null\n+++ blocker/new.txt\n@@ -0,0 +1 @@\n+hi\n";
        assert!(apply_patch(diff, 0, dir.path()).is_err());
        // Moved back rather than recreated, so owner, ACLs, and extended
        // attributes come back with it.
        assert_eq!(
            fs::metadata(dir.path().join("gone.txt")).unwrap().ino(),
            inode
        );
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("hk-deleted"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn a_deleted_file_leaves_no_copy_behind() {
        let dir = dir_with(&[("gone.txt", "x\n")]);
        let diff = "--- gone.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        assert_eq!(apply_patch(diff, 0, dir.path()), Ok(1));
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
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
                .to_string()
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

    #[cfg(unix)]
    #[test]
    fn roll_back_leaves_an_unchanged_file_alone() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir_with(&[("a.txt", "x\n")]);
        let path = dir.path().join("a.txt");
        // Read-only, so rewriting it would fail (unless run as root).
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        let original = (Some("x\n".to_string()), None);
        super::roll_back(&path, &original, None).unwrap();
        assert_eq!(read(&dir, "a.txt"), "x\n");
    }

    #[cfg(unix)]
    #[test]
    fn roll_back_reports_a_file_it_cannot_restore() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir_with(&[("a.txt", "")]);
        let path = dir.path().join("a.txt");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        let original = (Some("x\n".to_string()), None);
        let result = super::roll_back(&path, &original, Some("y\n"));
        // Root can write read-only files; everyone else gets the error.
        assert_eq!(result.is_err(), read(&dir, "a.txt").is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn roll_back_fails_for_a_file_it_cannot_read() {
        use std::os::unix::fs::PermissionsExt;
        let dir = dir_with(&[("created.txt", "hel")]);
        let path = dir.path().join("created.txt");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o200)).unwrap();
        // Root can read anything, and then the partial file is removed.
        let readable = fs::read(&path).is_ok();
        let result = super::roll_back(&path, &(None, None), Some("hello\n"));
        assert_eq!(result.is_err(), !readable);
        assert_eq!(path.exists(), !readable);
    }

    #[test]
    fn roll_back_restores_a_changed_file() {
        let dir = dir_with(&[("a.txt", "")]);
        let path = dir.path().join("a.txt");
        let original = (Some("x\n".to_string()), None);
        super::roll_back(&path, &original, Some("y\n")).unwrap();
        assert_eq!(read(&dir, "a.txt"), "x\n");
    }

    #[test]
    fn roll_back_removes_only_a_file_hk_was_creating() {
        let dir = dir_with(&[("partial.txt", "hel"), ("theirs.txt", "not ours\n")]);
        let absent = (None, None);
        super::roll_back(&dir.path().join("partial.txt"), &absent, Some("hello\n")).unwrap();
        assert!(!dir.path().join("partial.txt").exists());
        super::roll_back(&dir.path().join("theirs.txt"), &absent, Some("hello\n")).unwrap();
        assert_eq!(read(&dir, "theirs.txt"), "not ours\n");
    }

    #[test]
    fn output_without_a_patch_is_an_error() {
        let dir = dir_with(&[]);
        assert!(apply_patch("Issues were detected.\n", 0, dir.path()).is_err());
    }
}
