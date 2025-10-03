use crate::Result;
use std::fs;
use std::path::PathBuf;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, clap::Args)]
pub struct CheckByteOrderMarker {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl CheckByteOrderMarker {
    pub async fn run(&self) -> Result<()> {
        let mut found_bom = false;

        for file_path in &self.files {
            if has_bom(file_path)? {
                println!("{}", file_path.display());
                found_bom = true;
            }
        }

        if found_bom {
            std::process::exit(1);
        }

        Ok(())
    }
}

fn has_bom(path: &PathBuf) -> Result<bool> {
    // Read first 3 bytes to check for UTF-8 BOM
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(false), // File doesn't exist or can't be read
    };

    Ok(bytes.starts_with(UTF8_BOM))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_with_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("with_bom.txt");

        let mut content = UTF8_BOM.to_vec();
        content.extend_from_slice(b"Hello, world!");
        fs::write(&file, content).unwrap();

        let result = has_bom(&file).unwrap();
        assert!(result);
    }

    #[test]
    fn test_file_without_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("without_bom.txt");

        fs::write(&file, "Hello, world!").unwrap();

        let result = has_bom(&file).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_empty_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("empty.txt");

        fs::write(&file, "").unwrap();

        let result = has_bom(&file).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_file_shorter_than_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("short.txt");

        fs::write(&file, "Hi").unwrap();

        let result = has_bom(&file).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent");

        let result = has_bom(&file).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_file_with_partial_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("partial_bom.txt");

        // Only first 2 bytes of BOM
        fs::write(&file, &[0xEF, 0xBB, 0x00]).unwrap();

        let result = has_bom(&file).unwrap();
        assert!(!result);
    }
}
