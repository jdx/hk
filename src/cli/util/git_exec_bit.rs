//! Shared "is this file executable in git?" logic for the shebang checkers.
//!
//! The executable bit that matters is the one git records (the index mode),
//! not whatever the worktree filesystem happens to report. The worktree mode
//! is unreliable on Windows and on repos with `core.fileMode=false`.

use crate::Result;
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Whether the worktree filesystem's executable bit is trusted (`core.fileMode`).
pub fn file_mode_enabled() -> bool {
    let out = Command::new("git")
        .args(["config", "--type=bool", "core.fileMode"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim() != "false",
        _ => cfg!(unix),
    }
}

/// Maps an argument path to the key used by the index map: the path relative
/// to the repository top level (`top`), with `.`/`..` resolved lexically.
/// Paths outside `top` keep their absolute spelling and so are never found.
fn normalize(path: &Path, top: Option<&Path>) -> String {
    let resolved: PathBuf;
    let path = match (top, std::env::current_dir()) {
        (Some(top), Ok(cwd)) => {
            let abs = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            };
            let mut out = PathBuf::new();
            for c in abs.components() {
                match c {
                    Component::CurDir => {}
                    Component::ParentDir => {
                        out.pop();
                    }
                    other => out.push(other.as_os_str()),
                }
            }
            resolved = out;
            resolved.strip_prefix(top).unwrap_or(&resolved)
        }
        _ => path,
    };
    let s = path.to_string_lossy();
    // Only Windows uses `\` as a separator; on unix it is a legal filename
    // character and must match git's path exactly.
    #[cfg(windows)]
    let s: String = s.replace('\\', "/");
    #[cfg(not(windows))]
    let s: String = s.into_owned();
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

/// Index modes for every tracked file in the repository, keyed by the path
/// relative to the top level, plus that top level. Reading the whole index
/// (rather than passing the files as pathspecs) means a path outside the
/// repository can never make git fail the run; it is simply not found and
/// treated as untracked, and `../x` from a subdirectory resolves correctly.
/// `None` means we are not in a git repository (worktree modes are used).
fn index_modes() -> Result<Option<(PathBuf, HashMap<String, String>)>> {
    let top = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if top.status.success() {
        let top = PathBuf::from(String::from_utf8_lossy(&top.stdout).trim_end());
        let output = Command::new("git")
            .arg("-C")
            .arg(&top)
            .args(["ls-files", "-z", "--stage"])
            .output()?;
        if output.status.success() {
            return Ok(Some((top, parse_stage_output(&output.stdout))));
        }
        return Err(eyre::eyre!(
            "failed to read git index modes: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let output = top;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("not a git repository") {
        return Ok(None);
    }
    Err(eyre::eyre!(
        "failed to read git index modes: {}",
        stderr.trim()
    ))
}

fn ignore_case() -> bool {
    let out = Command::new("git")
        .args(["config", "--type=bool", "core.ignorecase"])
        .output();
    matches!(out, Ok(o) if o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
}

/// Resolves executability for each file: `Some(executable)` for regular
/// files, `None` for paths that are not regular files (directories,
/// symlinks, submodules) and so should be skipped.
///
/// Tracked files use the index mode. Untracked files use the worktree mode
/// when `core.fileMode` is trusted (unix only), otherwise "not executable"
/// (git would add them as 100644).
pub fn executable_flags(files: &[PathBuf]) -> Result<Vec<FileExec>> {
    let (top, modes) = match index_modes()? {
        Some((top, modes)) => (Some(top), Some(modes)),
        None => (None, None),
    };
    let trust_worktree = file_mode_enabled();
    // Built lazily: only on case-insensitive filesystems (core.ignorecase).
    let mut lower: Option<HashMap<String, String>> = None;
    files
        .iter()
        .map(|file| {
            let key = normalize(file, top.as_deref());
            let mut mode = modes.as_ref().and_then(|m| m.get(&key)).cloned();
            if mode.is_none()
                && let Some(m) = &modes
                && ignore_case_cached(&mut lower, m)
            {
                mode = lower
                    .as_ref()
                    .and_then(|l| l.get(&key.to_lowercase()))
                    .cloned();
            }
            let tracked = mode.is_some();
            // The worktree file type wins over the index: a tracked path
            // replaced by a symlink or directory is not a regular script.
            let metadata = std::fs::symlink_metadata(file)?;
            if !metadata.is_file() {
                return Ok(FileExec {
                    executable: None,
                    tracked,
                });
            }
            let executable = match mode.as_deref() {
                Some("100755") => Some(true),
                Some("100644") => Some(false),
                Some(_) => None,
                None => worktree_executable(&metadata, trust_worktree),
            };
            Ok(FileExec {
                executable,
                tracked,
            })
        })
        .collect()
}

/// Builds the lowercase index map on first use; returns whether `core.ignorecase` is on.
fn ignore_case_cached(
    lower: &mut Option<HashMap<String, String>>,
    modes: &HashMap<String, String>,
) -> bool {
    if lower.is_none() {
        *lower = Some(if ignore_case() {
            modes
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect()
        } else {
            HashMap::new()
        });
    }
    lower.as_ref().is_some_and(|l| !l.is_empty())
}

/// Executability of one file, and whether git tracks it.
pub struct FileExec {
    /// `None` for paths that are not regular files (skip them).
    pub executable: Option<bool>,
    pub tracked: bool,
}

#[allow(unused_variables)]
fn worktree_executable(metadata: &std::fs::Metadata, trust: bool) -> Option<bool> {
    if !trust {
        return Some(false);
    }
    #[cfg(unix)]
    {
        Some(metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        Some(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_dot_slash_and_keeps_unix_backslashes() {
        assert_eq!(normalize(Path::new("./a/b.sh"), None), "a/b.sh");
        #[cfg(unix)]
        assert_eq!(normalize(Path::new("a\\b.sh"), None), "a\\b.sh");
        #[cfg(windows)]
        assert_eq!(normalize(Path::new("a\\b.sh"), None), "a/b.sh");
    }

    #[test]
    fn parses_stage_output() {
        let out = b"100755 abc 0\ta b.sh\x00100644 def 0\tc.txt\x00120000 123 0\tl\x00";
        let m = parse_stage_output(out);
        assert_eq!(m["a b.sh"], "100755");
        assert_eq!(m["c.txt"], "100644");
        assert_eq!(m["l"], "120000");
    }
}
