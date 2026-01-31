use std::path::{Path, PathBuf};

use eyre::eyre;

use crate::Result;

/// Find the `.git` path from the current working directory by searching upward.
pub fn find_git_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    xx::file::find_up(&cwd, &[".git"])
        .ok_or_else(|| eyre!("No .git found in this or any parent directory"))
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
    // Resolve relative paths against the parent of the .git file
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
        // Canonicalize to clean up ../.. paths
        Ok(std::fs::canonicalize(&resolved).unwrap_or(resolved))
    } else {
        Ok(git_dir.to_path_buf())
    }
}

/// Given a `.git` path (found by find_up), resolve the hooks directory.
/// Git always looks for hooks in the **common** git directory, not the
/// worktree-specific one. So for worktrees we follow the `commondir` pointer.
/// - If `.git` is a directory (regular repo) → return `.git/hooks`
/// - If `.git` is a file (worktree) → resolve gitdir → resolve commondir → return `<common>/hooks`
pub fn resolve_git_hooks_dir(git_path: &Path) -> Result<PathBuf> {
    let git_dir = resolve_git_dir(git_path)?;
    let common_dir = resolve_common_git_dir(&git_dir)?;
    Ok(common_dir.join("hooks"))
}
