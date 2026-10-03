//! Shared "is this file executable in git?" logic for the shebang checkers.
//!
//! The executable bit that matters is the one git records (the index mode),
//! not whatever the worktree filesystem happens to report. The worktree mode
//! is unreliable on Windows and on repos with `core.fileMode=false`.

use crate::Result;
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Whether the worktree filesystem's executable bit is trusted (`core.fileMode`).
fn file_mode_enabled() -> bool {
    let out = Command::new("git")
        .args(["config", "--type=bool", "core.fileMode"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim() != "false",
        _ => cfg!(unix),
    }
}

fn normalize(path: &Path) -> String {
    let path = match std::env::current_dir() {
        Ok(cwd) if path.is_absolute() => path.strip_prefix(&cwd).unwrap_or(path),
        _ => path,
    };
    let s = path.to_string_lossy().replace('\\', "/");
    s.strip_prefix("./").unwrap_or(&s).to_string()
}

/// Parses `git ls-files -z --stage` output into path -> mode (e.g. `100755`).
fn parse_stage_output(out: &[u8]) -> HashMap<String, String> {
    let mut modes = HashMap::new();
    // Format: "<mode> <object> <stage>\t<path>\0"
    for entry in out.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let entry = String::from_utf8_lossy(entry);
        if let Some((meta, path)) = entry.split_once('\t')
            && let Some(mode) = meta.split(' ').next()
        {
            modes.insert(path.to_string(), mode.to_string());
        }
    }
    modes
}

/// Index modes for the tracked files among `files`, keyed by normalized path.
fn index_modes(files: &[PathBuf]) -> HashMap<String, String> {
    if files.is_empty() {
        return HashMap::new();
    }
    let output = Command::new("git")
        .args(["--literal-pathspecs", "ls-files", "-z", "--stage", "--"])
        .args(files)
        .output();
    match output {
        Ok(o) if o.status.success() => parse_stage_output(&o.stdout),
        _ => HashMap::new(),
    }
}

/// Resolves executability for each file: the index mode when the file is
/// tracked, otherwise the worktree mode when `core.fileMode` is trusted
/// (unix only), otherwise "not executable" (git would add it as 100644).
pub fn executable_flags(files: &[PathBuf]) -> Result<Vec<bool>> {
    let modes = index_modes(files);
    let trust_worktree = file_mode_enabled();
    files
        .iter()
        .map(|file| {
            if let Some(mode) = modes.get(&normalize(file)) {
                return Ok(mode.ends_with("755"));
            }
            worktree_executable(file, trust_worktree)
        })
        .collect()
}

#[allow(unused_variables)]
fn worktree_executable(path: &Path, trust: bool) -> Result<bool> {
    let metadata = std::fs::metadata(path)?;
    if metadata.is_dir() || !trust {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        Ok(metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_dot_slash_and_backslashes() {
        assert_eq!(normalize(Path::new("./a/b.sh")), "a/b.sh");
        assert_eq!(normalize(Path::new("a\\b.sh")), "a/b.sh");
    }

    #[test]
    fn parses_stage_output() {
        let out = b"100755 abc 0\ta b.sh\0100644 def 0\tc.txt\0120000 123 0\tl\0";
        let m = parse_stage_output(out);
        assert_eq!(m["a b.sh"], "100755");
        assert_eq!(m["c.txt"], "100644");
        assert_eq!(m["l"], "120000");
    }
}
