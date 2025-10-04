use crate::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct CheckSymlinks {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl CheckSymlinks {
    pub async fn run(&self) -> Result<()> {
        let mut found_broken = false;

        for file_path in &self.files {
            if is_broken_symlink(file_path)? {
                println!("{}", file_path.display());
                found_broken = true;
            }
        }

        if found_broken {
            return Err(eyre::eyre!("Broken symlinks found"));
        }

        Ok(())
    }
}

fn is_broken_symlink(path: &PathBuf) -> Result<bool> {
    // Check if path is a symlink
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return Ok(false), // File doesn't exist or can't be accessed
    };

    if !metadata.is_symlink() {
        return Ok(false);
    }

    // If it's a symlink, check if target exists
    // Using fs::metadata (not symlink_metadata) will follow the symlink
    match fs::metadata(path) {
        Ok(_) => Ok(false), // Target exists, symlink is valid
        Err(_) => Ok(true), // Target doesn't exist, symlink is broken
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    #[test]
    fn test_valid_symlink() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link");

        fs::write(&target, "content").unwrap();
        symlink(&target, &link).unwrap();

        let result = is_broken_symlink(&link).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_broken_symlink() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link");

        // Create symlink to non-existent target
        symlink(&target, &link).unwrap();

        let result = is_broken_symlink(&link).unwrap();
        assert!(result);
    }

    #[test]
    fn test_regular_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, "content").unwrap();

        let result = is_broken_symlink(&file).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent");

        let result = is_broken_symlink(&file).unwrap();
        assert!(!result); // Not a broken symlink, just doesn't exist
    }

    #[test]
    fn test_symlink_to_directory() {
        let dir = TempDir::new().unwrap();
        let target_dir = dir.path().join("target_dir");
        let link = dir.path().join("link");

        fs::create_dir(&target_dir).unwrap();
        symlink(&target_dir, &link).unwrap();

        let result = is_broken_symlink(&link).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_broken_symlink_to_directory() {
        let dir = TempDir::new().unwrap();
        let target_dir = dir.path().join("nonexistent_dir");
        let link = dir.path().join("link");

        symlink(&target_dir, &link).unwrap();

        let result = is_broken_symlink(&link).unwrap();
        assert!(result);
    }
}
