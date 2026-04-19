use std::path::{Path, PathBuf};

use eyre::eyre;

use crate::Result;

/// Find the `.git` path from the current working directory by searching upward.
///
/// Honors `GIT_DIR` if set (used by bare-repo dotfile managers like YADM), in
/// which case the returned path may be a bare repository directory rather than
/// a `.git` file/dir.
pub fn find_git_path() -> Result<PathBuf> {
    if let Some(git_dir) = std::env::var_os("GIT_DIR") {
        let p = PathBuf::from(&git_dir);
        let p = if p.is_absolute() {
            p
        } else {
            std::env::current_dir()?.join(p)
        };
        return Ok(p);
    }
    let cwd = std::env::current_dir()?;
    xx::file::find_up(&cwd, &[".git"])
        .ok_or_else(|| eyre!("No .git found in this or any parent directory"))
}

/// Return the effective working-tree root, honoring `GIT_WORK_TREE` when set
/// (for bare-repo setups like YADM). Falls back to walking up for `.git`, and
/// finally to `cwd` if no repository is found.
pub fn find_work_tree_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Some(wt) = std::env::var_os("GIT_WORK_TREE") {
        let p = PathBuf::from(&wt);
        return if p.is_absolute() { p } else { cwd.join(p) };
    }
    xx::file::find_up(&cwd, &[".git"])
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or(cwd)
}

/// Given a `.git` path (found by find_up), resolve the actual git directory.
/// - If `.git` is a directory → return it as-is
/// - If `.git` is a file (worktree) → read it, parse "gitdir: <path>", resolve that path
pub fn resolve_git_dir(git_path: &Path) -> Result<PathBuf> {
    if git_path.is_dir() {
        return Ok(git_path.to_path_buf());
    }
    // It's a file — parse the gitdir pointer
    let content = std::fs::read_to_string(git_path)
        .map_err(|e| eyre!("failed to read {}: {e}", git_path.display()))?;
    let gitdir = content
        .strip_prefix("gitdir: ")
        .map(|s| s.trim())
        .ok_or_else(|| eyre!("unexpected .git file format in {}", git_path.display()))?;
    let gitdir_path = PathBuf::from(gitdir);
    let resolved = if gitdir_path.is_absolute() {
        gitdir_path
    } else {
        git_path
            .parent()
            .ok_or_else(|| {
                eyre!(
                    "could not determine parent directory of .git file: {}",
                    git_path.display()
                )
            })?
            .join(&gitdir_path)
    };
    Ok(resolved)
}

/// Given the worktree-specific git dir (e.g. `.git/worktrees/<name>`), resolve
/// the common git directory by reading the `commondir` file if present.
/// Falls back to returning `git_dir` unchanged for regular repos.
fn resolve_common_git_dir(git_dir: &Path) -> Result<PathBuf> {
    let commondir_file = git_dir.join("commondir");
    if commondir_file.is_file() {
        let content = std::fs::read_to_string(&commondir_file)
            .map_err(|e| eyre!("failed to read {}: {e}", commondir_file.display()))?;
        let rel = content.trim();
        let resolved = git_dir.join(rel);
        Ok(std::fs::canonicalize(&resolved).unwrap_or(resolved))
    } else {
        Ok(git_dir.to_path_buf())
    }
}

/// Given a path like `.git/COMMIT_EDITMSG`, resolve it for worktrees.
/// In worktrees, `.git` is a file (not a directory), so paths like
/// `.git/COMMIT_EDITMSG` passed by git to hooks need to be resolved
/// through the actual git directory.
pub fn resolve_git_relative_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path.to_path_buf());
    }
    if let Ok(rest) = path.strip_prefix(".git") {
        let git_path = find_git_path()?;
        let git_dir = resolve_git_dir(&git_path)?;
        let resolved = git_dir.join(rest);
        if resolved.exists() {
            return Ok(resolved);
        }
    }
    Ok(path.to_path_buf())
}

/// Given a `.git` path (found by find_up), resolve the hooks directory.
/// Resolves to the common (shared) hooks dir, following the `commondir`
/// pointer for worktrees.
pub fn resolve_git_hooks_dir(git_path: &Path) -> Result<PathBuf> {
    let git_dir = resolve_git_dir(git_path)?;
    let common_dir = resolve_common_git_dir(&git_dir)?;
    Ok(common_dir.join("hooks"))
}

/// Returns the per-worktree hooks path if one is configured via
/// `git config --worktree core.hooksPath`. Returns None if not in a
/// worktree or if no per-worktree hooksPath is set.
///
/// Requires `extensions.worktreeConfig` to be enabled — without it,
/// `--worktree` falls back to `--local` which is not worktree-specific.
pub fn worktree_hooks_path() -> Option<PathBuf> {
    // --type=bool normalizes true/yes/on/1 to "true"
    let wt_config = std::process::Command::new("git")
        .args([
            "config",
            "--type=bool",
            "--get",
            "extensions.worktreeConfig",
        ])
        .output()
        .ok()?;
    if !wt_config.status.success() || String::from_utf8_lossy(&wt_config.stdout).trim() != "true" {
        return None;
    }

    let output = std::process::Command::new("git")
        .args(["config", "--worktree", "--get", "core.hooksPath"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}
