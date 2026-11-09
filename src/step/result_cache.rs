//! Remembers that a check passed, so the same check is not run again on
//! exactly the same file contents.
//!
//! A step opts in with `cache = true`. Only a check that exited successfully
//! is recorded, and only a check is ever skipped: a fixer always runs.
//!
//! An entry is an empty file named after a key. The key is a SHA-256 over the
//! hk version, the step's rendered command, working directory, environment,
//! shell and stdin, and the content (not the mtime) of the files the check
//! reads: the job's files, or, when the step sets `cache_inputs`, the files
//! those globs match. Entries are renamed into place, so concurrent hk
//! processes never see a partial entry, and the directory is pruned to
//! [`MAX_ENTRIES`] on write.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::{Result, env, glob};

/// Most entries kept; the oldest are deleted beyond it.
const MAX_ENTRIES: usize = 2000;
/// Entries not used for this long are deleted.
const MAX_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// What a check's result is keyed on, besides the contents of its files.
pub struct Inputs<'a> {
    pub step: &'a str,
    pub command: &'a str,
    pub dir: &'a Path,
    pub env: &'a [(String, String)],
    pub shell: Option<String>,
    pub stdin: Option<&'a str>,
    /// Files the check reads when the step lists no `cache_inputs`.
    pub files: &'a [PathBuf],
    /// Globs of files the whole check depends on, replacing `files`.
    pub cache_inputs: Option<&'a [String]>,
}

fn dir() -> PathBuf {
    env::HK_CACHE_DIR.join("results")
}

fn entry_path(key: &str) -> PathBuf {
    dir().join(key)
}

fn add(buf: &mut Vec<u8>, label: &str, value: &[u8]) {
    // Length-prefixed, so ("ab", "c") and ("a", "bc") hash differently.
    buf.extend((label.len() as u64).to_le_bytes());
    buf.extend(label.as_bytes());
    buf.extend((value.len() as u64).to_le_bytes());
    buf.extend(value);
}

fn add_file(buf: &mut Vec<u8>, path: &Path) -> Result<()> {
    let content = xx::hash::file_hash_sha256(path)?;
    add(buf, &path.to_string_lossy(), content.as_bytes());
    Ok(())
}

/// Every file in the work tree (tracked, or untracked and not ignored).
fn work_tree_files() -> Result<Vec<PathBuf>> {
    let output = std::process::Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()?;
    if !output.status.success() {
        eyre::bail!("git ls-files failed");
    }
    Ok(output
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| PathBuf::from(String::from_utf8_lossy(p).into_owned()))
        .collect())
}

/// The key for a check, or `None` when it cannot be keyed (a file cannot be
/// read, or git cannot list the inputs): such a check always runs.
pub fn key(inputs: &Inputs) -> Option<String> {
    key_result(inputs)
        .map_err(|err| debug!("{}: not caching: {err:#}", inputs.step))
        .ok()
}

fn key_result(inputs: &Inputs) -> Result<String> {
    let mut buf = Vec::new();
    add(&mut buf, "hk", env!("CARGO_PKG_VERSION").as_bytes());
    add(&mut buf, "step", inputs.step.as_bytes());
    add(&mut buf, "command", inputs.command.as_bytes());
    add(&mut buf, "dir", inputs.dir.to_string_lossy().as_bytes());
    for (k, v) in inputs.env {
        add(&mut buf, &format!("env:{k}"), v.as_bytes());
    }
    add(
        &mut buf,
        "shell",
        inputs.shell.as_deref().unwrap_or("").as_bytes(),
    );
    add(&mut buf, "stdin", inputs.stdin.unwrap_or("").as_bytes());
    let mut files = match inputs.cache_inputs {
        Some(globs) => glob::get_matches(globs, &work_tree_files()?)?,
        None => inputs.files.to_vec(),
    };
    files.sort();
    files.dedup();
    for file in &files {
        add_file(&mut buf, file)?;
    }
    Ok(xx::hash::sha256(&buf))
}

/// Whether a check with this key already passed. A hit refreshes the entry's
/// age so a check that keeps being skipped is not pruned.
pub fn hit(key: &str) -> bool {
    if !*env::HK_CACHE {
        return false;
    }
    let path = entry_path(key);
    if !path.is_file() {
        return false;
    }
    if let Ok(file) = std::fs::File::options().write(true).open(&path) {
        let _ = file.set_modified(SystemTime::now());
    }
    true
}

/// Records that the check with this key passed.
pub fn record(key: &str) {
    if !*env::HK_CACHE {
        return;
    }
    if let Err(err) = write(key) {
        warn!("failed to write a step result: {err:#}");
    }
}

fn write(key: &str) -> Result<()> {
    let dir = dir();
    std::fs::create_dir_all(&dir)?;
    let path = entry_path(key);
    let tmp = dir.join(format!("{key}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, b"")?;
    if let Err(err) = std::fs::rename(&tmp, &path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err.into());
    }
    prune(&dir, MAX_ENTRIES, MAX_AGE);
    Ok(())
}

/// Deletes entries past `max_age`, then the oldest beyond `max_entries`.
/// Another process may delete the same entry first; that is not an error.
fn prune(dir: &Path, max_entries: usize, max_age: Duration) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let now = SystemTime::now();
    let mut entries = vec![];
    for entry in read.flatten() {
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        let age = now.duration_since(modified).unwrap_or_default();
        let name = entry.file_name();
        let is_tmp = name.to_string_lossy().ends_with(".tmp");
        // A temporary file is only left behind by a crash; give a live
        // writer a minute before treating it as one.
        if age > max_age || (is_tmp && age > Duration::from_secs(60)) {
            let _ = std::fs::remove_file(entry.path());
        } else if !is_tmp {
            entries.push((modified, entry.path()));
        }
    }
    if entries.len() > max_entries {
        entries.sort();
        let excess = entries.len() - max_entries;
        for (_, path) in entries.into_iter().take(excess) {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs<'a>(command: &'a str, files: &'a [PathBuf]) -> Inputs<'a> {
        Inputs {
            step: "s",
            command,
            dir: Path::new("/repo"),
            env: &[],
            shell: None,
            stdin: None,
            files,
            cache_inputs: None,
        }
    }

    #[test]
    fn key_follows_content_and_command_not_mtime() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("a.txt");
        std::fs::write(&f, "one").unwrap();
        let files = [f.clone()];
        let k1 = key(&inputs("lint", &files)).unwrap();
        // Rewriting the same bytes changes the mtime only.
        std::fs::write(&f, "one").unwrap();
        assert_eq!(k1, key(&inputs("lint", &files)).unwrap());
        assert_ne!(k1, key(&inputs("lint --strict", &files)).unwrap());
        std::fs::write(&f, "two").unwrap();
        assert_ne!(k1, key(&inputs("lint", &files)).unwrap());
    }

    #[test]
    fn unreadable_file_is_not_cached() {
        let files = [PathBuf::from("/nonexistent/hk-result-cache")];
        assert!(key(&inputs("lint", &files)).is_none());
    }

    #[test]
    fn prune_drops_the_oldest_beyond_the_limit() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..5 {
            let p = tmp.path().join(format!("k{i}"));
            std::fs::write(&p, "").unwrap();
            let f = std::fs::File::options().write(true).open(&p).unwrap();
            f.set_modified(SystemTime::now() - Duration::from_secs(100 - i))
                .unwrap();
        }
        prune(tmp.path(), 3, MAX_AGE);
        let mut left: Vec<_> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["k2", "k3", "k4"]);
    }
}
