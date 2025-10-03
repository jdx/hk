use crate::Result;
use std::fs;
use std::path::PathBuf;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

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
    use tempfile::NamedTempFile;

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
    fn test_file_without_bom_unchanged() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"Hello, world!").unwrap();

        remove_bom(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_empty_file() {
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
