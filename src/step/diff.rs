//! Applying unified diffs directly to files.
//!
//! When a step has `check_diff` configured, instead of running the fixer command,
//! hk can apply the diff output directly using `git apply`. This is often faster
//! than running the fixer, especially for tools that are slow to start.

use crate::Result;
use eyre::{Context, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};

use super::normalize_diff_paths;
use super::types::Step;

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
    let mut out: Vec<String> = Vec::new();
    for line in diff.lines() {
        let rewritten = ["--- ", "+++ "].into_iter().find_map(|prefix| {
            let rest = line.strip_prefix(prefix)?;
            // Keep any tab-separated timestamp attached to the path.
            let (path, tail) = match rest.split_once('\t') {
                Some((p, t)) => (p, Some(t)),
                None => (rest, None),
            };
            let rel = Path::new(path).strip_prefix(base).ok()?.to_str()?;
            Some(match tail {
                Some(t) => format!("{prefix}{rel}\t{t}"),
                None => format!("{prefix}{rel}"),
            })
        });
        out.push(rewritten.unwrap_or_else(|| line.to_string()));
    }
    out.join("\n") + "\n"
}

/// Keep the patch in a file so even a large diff cannot deadlock against git's
/// stderr pipe, and a failed stdin write cannot leave git running during rollback.
fn git_apply(
    patch: &tempfile::NamedTempFile,
    base: &Path,
    args: &[&str],
) -> std::io::Result<Output> {
    Command::new("git")
        .arg("apply")
        .args(args)
        .args(["--whitespace=nowarn", "-"])
        .current_dir(base)
        .stdin(patch.reopen()?)
        .output()
}

/// `-z` preserves pathname bytes, including quoted/non-UTF-8 Git paths on Unix.
fn git_path(bytes: &[u8]) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(OsString::from_vec(bytes.to_vec()).into())
    }
    #[cfg(not(unix))]
    {
        Ok(OsString::from(std::str::from_utf8(bytes)?).into())
    }
}

#[derive(Debug)]
enum FileState {
    Absent,
    File(fs::Permissions),
    Directory(fs::Permissions),
    Symlink {
        target: PathBuf,
        #[cfg(windows)]
        directory: bool,
    },
}

struct DiffBackup {
    dir: tempfile::TempDir,
    root: PathBuf,
    entries: BTreeMap<PathBuf, FileState>,
}

impl DiffBackup {
    fn create(patch: &tempfile::NamedTempFile, base: &Path, strip: &str) -> Result<Self> {
        let mut paths = BTreeSet::new();
        // Let git parse names and strip levels. Reverse stats include the source
        // of renames/copies; forward stats include their destination. A preflight
        // rejects invalid patches, but cannot replace rollback for write errors.
        for args in [
            vec![strip, "--numstat", "-z", "--check"],
            vec![strip, "--numstat", "-z", "--reverse"],
        ] {
            let output = git_apply(patch, base, &args)?;
            if !output.status.success() {
                bail!(
                    "cannot determine diff targets: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            for record in output.stdout.split(|&b| b == 0).filter(|s| !s.is_empty()) {
                let name = record
                    .splitn(3, |&b| b == b'\t')
                    .nth(2)
                    .ok_or_else(|| eyre::eyre!("invalid diff stats"))?;
                let mut path = PathBuf::new();
                for component in git_path(name)?.components() {
                    match component {
                        Component::Normal(part) => path.push(part),
                        Component::CurDir => {}
                        _ => bail!("unsafe diff path"),
                    }
                }
                // Git may remove empty parents on deletion, or create parents
                // for new files. Capture those too, including file/dir changes.
                for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
                    paths.insert(ancestor.to_path_buf());
                }
            }
        }
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(base)
            .output()?;
        let root = if output.status.success() {
            let name = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
            #[cfg(windows)]
            let name = name.strip_suffix(b"\r").unwrap_or(name);
            git_path(name)?.canonicalize()?
        } else {
            base.to_path_buf()
        };
        let mut backup = Self {
            dir: tempfile::Builder::new()
                .prefix("hk-diff-backup-")
                .tempdir()?,
            root,
            entries: BTreeMap::new(),
        };
        fs::create_dir(backup.dir.path().join("files"))?;
        // Path ordering visits parents before children. Never follow an original
        // symlink (or file) when capturing paths that a patch turns into children.
        for path in paths {
            let parent_is_directory = path
                .parent()
                .and_then(|parent| backup.entries.get(parent))
                .is_none_or(|state| matches!(state, FileState::Directory(_)));
            let state = if parent_is_directory {
                backup.capture(&path)?
            } else {
                FileState::Absent
            };
            backup.entries.insert(path, state);
        }
        // Retain the complete recovery instructions if any restoration fails.
        // Debug escaping keeps unusual path bytes unambiguous in this manifest.
        fs::write(
            backup.dir.path().join("manifest.txt"),
            format!(
                "Original paths relative to {:?}\nFile contents and symlink targets are in files/.\n{:#?}\n",
                backup.root, backup.entries
            ),
        )?;
        Ok(backup)
    }

    fn capture(&self, path: &Path) -> Result<FileState> {
        let source = self.root.join(path);
        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(FileState::Absent),
            Err(err) => return Err(err.into()),
        };
        let saved = self.dir.path().join("files").join(path);
        fs::create_dir_all(saved.parent().unwrap())?;
        if metadata.is_dir() {
            fs::create_dir_all(saved)?;
            Ok(FileState::Directory(metadata.permissions()))
        } else if metadata.is_file() {
            fs::copy(&source, &saved)
                .wrap_err_with(|| format!("cannot back up {}", source.display()))?;
            Ok(FileState::File(metadata.permissions()))
        } else if metadata.file_type().is_symlink() {
            let target = fs::read_link(&source)?;
            // Save link text, not its referent. This also avoids requiring the
            // ability to create symlinks merely to take a backup on Windows.
            fs::write(saved, target.as_os_str().as_encoded_bytes())?;
            Ok(FileState::Symlink {
                target,
                #[cfg(windows)]
                directory: {
                    use std::os::windows::fs::FileTypeExt;
                    metadata.file_type().is_symlink_dir()
                },
            })
        } else {
            bail!("unsupported diff target: {}", source.display())
        }
    }

    /// Check each parent without following links that a partial patch may have
    /// introduced. Such parents must be restored before their children.
    fn parents_are_directories(&self, path: &Path) -> Result<bool> {
        let mut parent = self.root.clone();
        for component in path.parent().unwrap().components() {
            parent.push(component);
            match fs::symlink_metadata(&parent) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => return Ok(false),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
                Err(err) => return Err(err.into()),
            }
        }
        Ok(true)
    }

    fn restore(self) -> Result<()> {
        let mut errors = Vec::new();
        // Remove newly created children before parents, and clear type changes.
        // Never recursively remove directories: unrelated files must survive.
        for (path, state) in self.entries.iter().rev() {
            if let Err(err) = self.remove_created(path, state) {
                errors.push(format!("{}: {err:#}", self.root.join(path).display()));
            }
        }
        for (path, state) in &self.entries {
            if let Err(err) = self.restore_entry(path, state) {
                errors.push(format!("{}: {err:#}", self.root.join(path).display()));
            }
        }
        // Restore directory permissions last, after recreating their children.
        for (path, state) in self.entries.iter().rev() {
            if let FileState::Directory(permissions) = state {
                let result = (|| -> Result<()> {
                    if !self.parents_are_directories(path)? {
                        bail!("parent directory has not been restored");
                    }
                    let dest = self.root.join(path);
                    let metadata = fs::symlink_metadata(&dest)?;
                    if !metadata.is_dir() {
                        bail!("directory has not been restored");
                    }
                    if metadata.permissions() != *permissions {
                        fs::set_permissions(dest, permissions.clone())?;
                    }
                    Ok(())
                })();
                if let Err(err) = result {
                    errors.push(format!("{}: {err:#}", self.root.join(path).display()));
                }
            }
        }
        if !errors.is_empty() {
            let backup_dir = self.dir.keep();
            bail!(
                "failed to restore files after git apply; refusing to run fixer. Original files retained in {}:\n{}",
                backup_dir.display(),
                errors.join("\n")
            );
        }
        Ok(())
    }

    fn remove_created(&self, path: &Path, state: &FileState) -> Result<()> {
        if !self.parents_are_directories(path)? {
            return Ok(());
        }
        let dest = self.root.join(path);
        let metadata = match fs::symlink_metadata(&dest) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err.into()),
        };
        let remove = match state {
            FileState::Absent => true,
            FileState::Directory(_) => !metadata.is_dir(),
            _ => metadata.is_dir(),
        };
        if remove {
            if metadata.is_dir() {
                fs::remove_dir(dest)?;
            } else {
                remove_file_or_symlink(&dest)?;
            }
        }
        Ok(())
    }

    fn restore_entry(&self, path: &Path, state: &FileState) -> Result<()> {
        if matches!(state, FileState::Absent) {
            return Ok(());
        }
        if !self.parents_are_directories(path)? {
            bail!("parent directory has not been restored");
        }
        let dest = self.root.join(path);
        let parent = dest.parent().unwrap();
        match state {
            FileState::File(permissions) => {
                let content = fs::read(self.dir.path().join("files").join(path))?;
                if let Ok(metadata) = fs::symlink_metadata(&dest)
                    && metadata.is_file()
                    && metadata.permissions() == *permissions
                    && fs::read(&dest).is_ok_and(|current| current == content)
                {
                    return Ok(());
                }
                // Prepare the complete original before replacing a surviving
                // file or symlink, so a failed restore cannot truncate its data.
                let mut restored = tempfile::NamedTempFile::new_in(parent)?;
                restored.write_all(&content)?;
                restored.as_file().set_permissions(permissions.clone())?;
                restored.persist(&dest)?;
            }
            FileState::Directory(_) => match fs::symlink_metadata(&dest) {
                Ok(metadata) if metadata.is_dir() => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&dest)?,
                _ => bail!("cannot restore directory"),
            },
            FileState::Symlink { target, .. } => {
                if fs::read_link(&dest).is_ok_and(|current| current == *target) {
                    return Ok(());
                }
                let staging = tempfile::tempdir_in(parent)?;
                let link = staging.path().join("link");
                #[cfg(unix)]
                std::os::unix::fs::symlink(target, &link)?;
                #[cfg(windows)]
                if let FileState::Symlink { directory, .. } = state {
                    if *directory {
                        std::os::windows::fs::symlink_dir(target, &link)?;
                    } else {
                        std::os::windows::fs::symlink_file(target, &link)?;
                    }
                }
                fs::rename(link, dest)?;
            }
            FileState::Absent => {}
        }
        Ok(())
    }
}

fn remove_file_or_symlink(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        if fs::symlink_metadata(path)?.file_type().is_symlink_dir() {
            return fs::remove_dir(path);
        }
    }
    fs::remove_file(path)
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
    /// * `Ok(false)` - No changes remain from git apply; caller may run the fixer
    /// * `Err(_)` - I/O or restoration failed; caller must stop
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

        // Detect if this diff uses a/ and b/ prefixes (git-style)
        // Use -p1 to strip prefixes if present, -p0 otherwise
        let mut has_a_prefix = false;
        let mut has_b_prefix = false;
        for line in diff_content.lines() {
            if line.starts_with("--- a/") {
                has_a_prefix = true;
            } else if line.starts_with("+++ b/") {
                has_b_prefix = true;
            }
            if has_a_prefix && has_b_prefix {
                break;
            }
        }
        let strip_level = if has_a_prefix && has_b_prefix {
            "-p1"
        } else {
            "-p0"
        };

        let prepared = (|| -> Result<_> {
            let mut patch = tempfile::NamedTempFile::new()?;
            patch.write_all(diff_content.as_bytes())?;
            let backup = DiffBackup::create(&patch, &base, strip_level)?;
            Ok((patch, backup))
        })();
        let (patch, backup) = match prepared {
            Ok(prepared) => prepared,
            Err(err) => {
                debug!("{}: cannot safely apply diff: {err:#}", self.name);
                return Ok(false);
            }
        };

        let output = git_apply(&patch, &base, &[strip_level]);
        if let Ok(output) = &output
            && output.status.success()
        {
            debug!("{}: successfully applied diff", self.name);
            return Ok(true);
        }
        match output {
            Ok(output) => debug!(
                "{}: git apply failed: {}",
                self.name,
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(err) => warn!("{}: git apply failed: {err}", self.name),
        }
        // git apply can remove several files before failing to recreate one.
        // Only permit fallback once every original has been restored.
        backup.restore()?;
        Ok(false)
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
    fn does_not_touch_diff_body_lines() {
        let diff = "--- /w/svc/a.go\n+++ /w/svc/a.go\n@@ -1 +1 @@\n---- not a header\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/w/svc")),
            "--- a.go\n+++ a.go\n@@ -1 +1 @@\n---- not a header\n"
        );
    }
}

#[cfg(test)]
mod apply_diff_tests {
    use super::*;

    fn patch() -> tempfile::NamedTempFile {
        let mut patch = tempfile::NamedTempFile::new().unwrap();
        patch
            .write_all(b"--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-before\n+after\n")
            .unwrap();
        patch
    }

    #[test]
    fn backs_up_subdirectory_paths_relative_to_repository_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .current_dir(&root)
                .status()
                .unwrap()
                .success()
        );
        let base = root.join("sub");
        fs::create_dir(&base).unwrap();
        fs::write(base.join("file.txt"), b"before\n").unwrap();
        let patch = patch();
        let backup = DiffBackup::create(&patch, &base, "-p1").unwrap();
        assert!(git_apply(&patch, &base, &["-p1"]).unwrap().status.success());
        assert_eq!(fs::read(base.join("file.txt")).unwrap(), b"after\n");
        backup.restore().unwrap();
        assert_eq!(fs::read(base.join("file.txt")).unwrap(), b"before\n");
    }

    #[test]
    fn backs_up_the_target_git_selects_without_a_repository() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        fs::write(base.join("old.txt"), b"before\n").unwrap();
        fs::write(base.join("new.txt"), b"before\n").unwrap();
        let mut patch = tempfile::NamedTempFile::new().unwrap();
        patch
            .write_all(b"--- old.txt\n+++ new.txt\n@@ -1 +1 @@\n-before\n+after\n")
            .unwrap();
        let backup = DiffBackup::create(&patch, &base, "-p0").unwrap();
        assert_eq!(
            backup.entries.keys().cloned().collect::<Vec<_>>(),
            vec![PathBuf::from("new.txt")]
        );
        assert!(git_apply(&patch, &base, &["-p0"]).unwrap().status.success());
        assert_eq!(fs::read(base.join("new.txt")).unwrap(), b"after\n");
        backup.restore().unwrap();
        assert_eq!(fs::read(base.join("old.txt")).unwrap(), b"before\n");
        assert_eq!(fs::read(base.join("new.txt")).unwrap(), b"before\n");
    }

    #[derive(Debug, PartialEq)]
    enum TreeEntry {
        File(Vec<u8>, fs::Permissions),
        Directory(fs::Permissions),
        Symlink(PathBuf),
    }

    fn tree(root: &Path) -> BTreeMap<PathBuf, TreeEntry> {
        fn visit(root: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, TreeEntry>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let metadata = fs::symlink_metadata(&path).unwrap();
                let state = if metadata.is_dir() {
                    visit(root, &path, entries);
                    TreeEntry::Directory(metadata.permissions())
                } else if metadata.file_type().is_symlink() {
                    TreeEntry::Symlink(fs::read_link(&path).unwrap())
                } else {
                    TreeEntry::File(fs::read(&path).unwrap(), metadata.permissions())
                };
                entries.insert(path.strip_prefix(root).unwrap().to_path_buf(), state);
            }
        }
        let mut entries = BTreeMap::new();
        visit(root, root, &mut entries);
        entries
    }

    /// Compare successful application with upstream's git apply command, then
    /// restore and compare the entire original tree (including absent paths).
    fn assert_compatible_and_restorable(diff: &str, setup: impl Fn(&Path)) {
        let expected_dir = tempfile::tempdir().unwrap();
        let expected = expected_dir.path().canonicalize().unwrap();
        let actual_dir = tempfile::tempdir().unwrap();
        let actual = actual_dir.path().canonicalize().unwrap();
        setup(&expected);
        setup(&actual);
        let original = tree(&actual);
        let mut patch = tempfile::NamedTempFile::new().unwrap();
        patch.write_all(diff.as_bytes()).unwrap();
        let output = git_apply(&patch, &expected, &["-p0"]).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let backup = DiffBackup::create(&patch, &actual, "-p0").unwrap();
        assert!(
            Step::default()
                .apply_diff_output(diff, actual.to_str())
                .unwrap()
        );
        assert_eq!(tree(&actual), tree(&expected));
        backup.restore().unwrap();
        assert_eq!(tree(&actual), original);
    }

    #[test]
    fn preserves_creation_deletion_rename_copy_and_directory_changes() {
        for diff in [
            "--- /dev/null\n+++ new/nested/file.txt\n@@ -0,0 +1 @@\n+new\n",
            "--- old.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-before\n",
            "diff --git old.txt new/nested/file.txt\nsimilarity index 100%\nrename from old.txt\nrename to new/nested/file.txt\n",
            "diff --git old.txt new/nested/file.txt\nsimilarity index 100%\ncopy from old.txt\ncopy to new/nested/file.txt\n",
            "--- old.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-before\n--- /dev/null\n+++ old.txt/child.txt\n@@ -0,0 +1 @@\n+new\n",
            "--- nested/child.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-before\n--- /dev/null\n+++ nested\n@@ -0,0 +1 @@\n+new\n",
            "diff --git old.txt new.txt\nsimilarity index 100%\nrename from old.txt\nrename to new.txt\ndiff --git other.txt old.txt\nsimilarity index 100%\nrename from other.txt\nrename to old.txt\n",
        ] {
            assert_compatible_and_restorable(diff, |root| {
                fs::write(root.join("old.txt"), b"before\n").unwrap();
                fs::write(root.join("other.txt"), b"other\n").unwrap();
                fs::create_dir(root.join("nested")).unwrap();
                fs::write(root.join("nested/child.txt"), b"before\n").unwrap();
                fs::write(root.join("untouched.txt"), b"keep\n").unwrap();
            });
        }
    }

    #[cfg(unix)]
    #[test]
    fn preserves_executable_mode_changes() {
        use std::os::unix::fs::PermissionsExt;
        assert_compatible_and_restorable(
            "diff --git script.sh script.sh\nold mode 100644\nnew mode 100755\n",
            |root| {
                fs::write(root.join("script.sh"), b"#!/bin/sh\n").unwrap();
                fs::set_permissions(root.join("script.sh"), fs::Permissions::from_mode(0o644))
                    .unwrap();
            },
        );
    }

    #[cfg(unix)]
    #[test]
    fn preserves_symlink_changes_without_touching_referents() {
        for diff in [
            "diff --git link link\nnew file mode 120000\n--- /dev/null\n+++ link\n@@ -0,0 +1 @@\n+target\n\\ No newline at end of file\n",
            "diff --git old-link old-link\ndeleted file mode 120000\n--- old-link\n+++ /dev/null\n@@ -1 +0,0 @@\n-target\n\\ No newline at end of file\n",
            "diff --git old-link old-link\n--- old-link\n+++ old-link\n@@ -1 +1 @@\n-target\n\\ No newline at end of file\n+missing\n\\ No newline at end of file\n",
            "diff --git old-link renamed-link\nsimilarity index 100%\nrename from old-link\nrename to renamed-link\n",
            "diff --git old-link old-link\ndeleted file mode 120000\n--- old-link\n+++ /dev/null\n@@ -1 +0,0 @@\n-target\n\\ No newline at end of file\n--- /dev/null\n+++ old-link/child\n@@ -0,0 +1 @@\n+new\n",
        ] {
            assert_compatible_and_restorable(diff, |root| {
                fs::write(root.join("target"), b"keep me\n").unwrap();
                std::os::unix::fs::symlink("target", root.join("old-link")).unwrap();
            });
        }
    }

    #[cfg(unix)]
    #[test]
    fn preserves_git_quoted_names() {
        let header = r#""tab\tname""#;
        let diff = format!(
            "diff --git {header} {header}\n--- {header}\n+++ {header}\n@@ -1 +1 @@\n-before\n+after\n"
        );
        assert_compatible_and_restorable(&diff, |root| {
            fs::write(root.join("tab\tname"), b"before\n").unwrap();
        });
    }

    // macOS filesystems reject invalid UTF-8 filenames before Git can use them.
    #[cfg(target_os = "linux")]
    #[test]
    fn preserves_non_utf8_names() {
        use std::os::unix::ffi::OsStringExt;
        let header = r#""invalid\377""#;
        let diff = format!(
            "diff --git {header} {header}\n--- {header}\n+++ {header}\n@@ -1 +1 @@\n-before\n+after\n"
        );
        assert_compatible_and_restorable(&diff, |root| {
            fs::write(
                root.join(OsString::from_vec(b"invalid\xff".to_vec())),
                b"before\n",
            )
            .unwrap();
        });
    }

    #[test]
    fn preserves_binary_patches() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        fs::write(dir.path().join("data.bin"), b"\0before\xff").unwrap();
        assert!(
            Command::new("git")
                .args(["add", "data.bin"])
                .current_dir(dir.path())
                .status()
                .unwrap()
                .success()
        );
        fs::write(dir.path().join("data.bin"), b"\0after\xff").unwrap();
        let output = Command::new("git")
            .args(["diff", "--binary", "--no-prefix"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        let diff = std::str::from_utf8(&output.stdout).unwrap();
        assert!(diff.contains("GIT binary patch"));
        assert_compatible_and_restorable(diff, |root| {
            fs::write(root.join("data.bin"), b"\0before\xff").unwrap();
        });
    }

    #[test]
    fn failed_restoration_keeps_backups_and_restores_remaining_files() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().canonicalize().unwrap();
        let mut patch = patch();
        patch
            .write_all(b"--- a/other.txt\n+++ b/other.txt\n@@ -1 +1 @@\n-before\n+after\n")
            .unwrap();
        fs::write(base.join("file.txt"), b"before\n").unwrap();
        fs::write(base.join("other.txt"), b"before\n").unwrap();
        let backup = DiffBackup::create(&patch, &base, "-p1").unwrap();
        let saved = backup.dir.path().to_path_buf();
        fs::remove_file(base.join("file.txt")).unwrap();
        fs::create_dir(base.join("file.txt")).unwrap();
        fs::write(base.join("file.txt/unrelated"), b"keep me").unwrap();
        fs::remove_file(base.join("other.txt")).unwrap();
        let err = backup.restore().unwrap_err();
        assert!(err.to_string().contains("refusing to run fixer"));
        assert!(err.to_string().contains(&saved.display().to_string()));
        assert_eq!(fs::read(base.join("other.txt")).unwrap(), b"before\n");
        for name in ["file.txt", "other.txt"] {
            assert_eq!(
                fs::read(saved.join("files").join(name)).unwrap(),
                b"before\n"
            );
        }
        fs::remove_dir_all(saved).unwrap();
    }
}
