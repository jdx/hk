use crate::Result;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const DEFAULT_MAX_SIZE_KB: u64 = 500;

#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct CheckAddedLargeFiles {
    /// Maximum file size in kilobytes
    #[usage(long, default_value_t = DEFAULT_MAX_SIZE_KB, default = "500")]
    pub maxkb: u64,

    /// Files to check
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,
}

impl CheckAddedLargeFiles {
    pub async fn run(&self) -> Result<()> {
        let max_size_bytes = self.maxkb * 1024;
        let mut large = Vec::new();
        for file_path in &self.files {
            if is_too_large(file_path, max_size_bytes)? {
                large.push(file_path);
            }
        }

        // Git stores only a small pointer for a file tracked by Git LFS, so
        // its size on disk says nothing about what gets committed. Only the
        // oversized files are asked about, which keeps this to one git call
        // in the usual case.
        let lfs = lfs_tracked(&large);
        let mut found_large = false;
        for (file_path, is_lfs) in large.iter().zip(lfs) {
            if !is_lfs {
                println!("{}", file_path.display());
                found_large = true;
            }
        }

        if found_large {
            return Err(eyre::eyre!("Large files found"));
        }

        Ok(())
    }
}

fn is_too_large(path: &PathBuf, max_size: u64) -> Result<bool> {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(false), // File doesn't exist or can't be accessed
    };

    // Skip directories
    if metadata.is_dir() {
        return Ok(false);
    }

    Ok(metadata.len() > max_size)
}

/// Whether each file's `filter` attribute is `lfs`, in the order given.
///
/// git rejects the whole query if any path is outside the repository, so on
/// failure each file is asked about on its own. A file git can't answer for
/// (no git, not in a repository) counts as not tracked by LFS.
fn lfs_tracked(files: &[&PathBuf]) -> Vec<bool> {
    if files.is_empty() {
        return vec![];
    }
    if let Some(lfs) = check_attr_filter(files) {
        return lfs;
    }
    files
        .iter()
        .map(|f| {
            check_attr_filter(&[f])
                .and_then(|v| v.first().copied())
                .unwrap_or(false)
        })
        .collect()
}

fn check_attr_filter(files: &[&PathBuf]) -> Option<Vec<bool>> {
    let mut child = Command::new("git")
        .args(["check-attr", "-z", "--stdin", "filter"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut input = Vec::new();
    for f in files {
        input.extend_from_slice(f.as_os_str().as_encoded_bytes());
        input.push(0);
    }
    // Write from a thread so a large output can't deadlock against the input.
    let mut stdin = child.stdin.take()?;
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let output = child.wait_with_output().ok()?;
    writer.join().ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    let lfs = parse_check_attr(&output.stdout);
    (lfs.len() == files.len()).then_some(lfs)
}

/// Parses `git check-attr -z` output (`<path> NUL <attr> NUL <value> NUL`
/// per file, in input order) into whether each value is `lfs`.
fn parse_check_attr(stdout: &[u8]) -> Vec<bool> {
    let fields: Vec<&[u8]> = stdout.split(|&b| b == 0).collect();
    fields
        .as_chunks::<3>()
        .0
        .iter()
        .map(|[_, _, value]| *value == b"lfs")
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_small_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "small content").unwrap();

        let result = is_too_large(&file.path().to_path_buf(), 1024).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_large_file() {
        let file = NamedTempFile::new().unwrap();
        // Create a 2KB file
        let large_content = vec![b'x'; 2048];
        fs::write(file.path(), large_content).unwrap();

        let result = is_too_large(&file.path().to_path_buf(), 1024).unwrap();
        assert!(result);
    }

    #[test]
    fn test_file_exactly_at_limit() {
        let file = NamedTempFile::new().unwrap();
        // Create exactly 1024 bytes
        let content = vec![b'x'; 1024];
        fs::write(file.path(), content).unwrap();

        let result = is_too_large(&file.path().to_path_buf(), 1024).unwrap();
        assert!(!result); // Equal to limit is OK
    }

    #[test]
    fn test_file_one_byte_over_limit() {
        let file = NamedTempFile::new().unwrap();
        // Create 1025 bytes
        let content = vec![b'x'; 1025];
        fs::write(file.path(), content).unwrap();

        let result = is_too_large(&file.path().to_path_buf(), 1024).unwrap();
        assert!(result);
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "").unwrap();

        let result = is_too_large(&file.path().to_path_buf(), 1024).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_nonexistent_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("nonexistent");

        let result = is_too_large(&file, 1024).unwrap();
        assert!(!result);
    }

    #[test]
    fn parses_check_attr_output() {
        let out = b"a.bin\0filter\0lfs\0b.txt\0filter\0unspecified\0c.dat\0filter\0other\0";
        assert_eq!(parse_check_attr(out), vec![true, false, false]);
        assert_eq!(parse_check_attr(b""), Vec::<bool>::new());
    }

    #[test]
    fn test_directory() {
        let dir = tempfile::TempDir::new().unwrap();

        let result = is_too_large(&dir.path().to_path_buf(), 1024).unwrap();
        assert!(!result); // Directories should be skipped
    }
}
