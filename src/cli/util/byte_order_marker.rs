use crate::Result;
use std::fs;
use std::path::PathBuf;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, clap::Args)]
pub struct CheckByteOrderMarker {
    /// Output a diff of the change.
    #[clap(short, long)]
    pub diff: bool,

    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl CheckByteOrderMarker {
    pub async fn run(&self) -> Result<()> {
        let mut found_bom = false;

        for file_path in &self.files {
            if let Some(content) = read_if_has_bom(file_path)? {
                if self.diff {
                    print!("{}", generate_diff(file_path, &content));
                } else {
                    println!("{}", file_path.display());
                }
                found_bom = true;
            }
        }

        if found_bom {
            return Err(eyre::eyre!("Files with BOM found"));
        }

        Ok(())
    }
}

#[derive(Debug, clap::Args)]
pub struct FixByteOrderMarker {
    /// Files to remove BOM from
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl FixByteOrderMarker {
    pub async fn run(&self) -> Result<()> {
        for file_path in &self.files {
            remove_bom(file_path)?;
        }

        Ok(())
    }
}

/// Returns the file content if it has a BOM, None otherwise
fn read_if_has_bom(path: &PathBuf) -> Result<Option<Vec<u8>>> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return Ok(None),
    };

    if bytes.starts_with(UTF8_BOM) {
        Ok(Some(bytes))
    } else {
        Ok(None)
    }
}

fn generate_diff(path: &PathBuf, content_with_bom: &[u8]) -> String {
    let original = String::from_utf8_lossy(content_with_bom);
    let without_bom = String::from_utf8_lossy(&content_with_bom[UTF8_BOM.len()..]);
    let path_str = path.display().to_string();
    crate::diff::render_unified_diff(
        &original,
        &without_bom,
        &format!("a/{}", path_str),
        &format!("b/{}", path_str),
    )
}

fn remove_bom(path: &PathBuf) -> Result<()> {
    let content = fs::read(path)?;

    if content.starts_with(UTF8_BOM) {
        // Remove the first 3 bytes (the BOM)
        let without_bom = &content[3..];
        fs::write(path, without_bom)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_file_with_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("with_bom.txt");

        let mut content = UTF8_BOM.to_vec();
        content.extend_from_slice(b"Hello, world!");
        fs::write(&file, content).unwrap();

        let result = read_if_has_bom(&file).unwrap().is_some();
        assert!(result);
    }

    #[test]
    fn test_file_without_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("without_bom.txt");

        fs::write(&file, "Hello, world!").unwrap();

        let result = read_if_has_bom(&file).unwrap().is_none();
        assert!(result);
    }

    #[test]
    fn test_empty_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("empty.txt");

        fs::write(&file, "").unwrap();

        let result = read_if_has_bom(&file).unwrap().is_none();
        assert!(result);
    }

    #[test]
    fn test_file_shorter_than_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("short.txt");

        fs::write(&file, "Hi").unwrap();

        let result = read_if_has_bom(&file).unwrap().is_none();
        assert!(result);
    }

    #[test]
    fn test_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent");

        let result = read_if_has_bom(&file).unwrap().is_none();
        assert!(result);
    }

    #[test]
    fn test_file_with_partial_bom() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("partial_bom.txt");

        // Only first 2 bytes of BOM
        fs::write(&file, [0xEF, 0xBB, 0x00]).unwrap();

        let result = read_if_has_bom(&file).unwrap().is_none();
        assert!(result);
    }

    #[test]
    fn test_remove_bom() {
        let file = NamedTempFile::new().unwrap();

        let mut content = UTF8_BOM.to_vec();
        content.extend_from_slice(b"Hello, world!");
        fs::write(file.path(), &content).unwrap();

        remove_bom(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_remove_bom_file_without_bom_unchanged() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"Hello, world!").unwrap();

        remove_bom(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_remove_bom_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"").unwrap();

        remove_bom(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"");
    }

    #[test]
    fn test_file_only_bom() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), UTF8_BOM).unwrap();

        remove_bom(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"");
    }
}
