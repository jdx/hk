use crate::Result;
use std::fs;
use std::path::PathBuf;

const DEFAULT_MAX_SIZE_KB: u64 = 500;

#[derive(Debug, clap::Args)]
pub struct CheckAddedLargeFiles {
    /// Maximum file size in kilobytes (default: 500)
    #[clap(long, default_value_t = DEFAULT_MAX_SIZE_KB)]
    pub maxkb: u64,

    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl CheckAddedLargeFiles {
    pub async fn run(&self) -> Result<()> {
        let max_size_bytes = self.maxkb * 1024;
        let mut found_large = false;

        for file_path in &self.files {
            if is_too_large(file_path, max_size_bytes)? {
                println!("{}", file_path.display());
                found_large = true;
            }
        }

        if found_large {
            std::process::exit(1);
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
    fn test_directory() {
        let dir = tempfile::TempDir::new().unwrap();

        let result = is_too_large(&dir.path().to_path_buf(), 1024).unwrap();
        assert!(!result); // Directories should be skipped
    }
}
