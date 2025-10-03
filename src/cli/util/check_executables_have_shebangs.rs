use crate::Result;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct CheckExecutablesHaveShebangs {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl CheckExecutablesHaveShebangs {
    pub async fn run(&self) -> Result<()> {
        let mut found_issues = false;

        for file_path in &self.files {
            if is_executable(file_path)? && !has_shebang(file_path)? {
                println!("{}", file_path.display());
                found_issues = true;
            }
        }

        if found_issues {
            std::process::exit(1);
        }

        Ok(())
    }
}

fn is_executable(path: &PathBuf) -> Result<bool> {
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();

    // Check if any execute bit is set
    Ok(permissions.mode() & 0o111 != 0)
}

fn has_shebang(path: &PathBuf) -> Result<bool> {
    let content = fs::read(path)?;

    // Skip binary files
    if content.contains(&0) {
        return Ok(true); // Don't flag binary files as missing shebangs
    }

    // Check if file starts with #!
    Ok(content.len() >= 2 && content[0] == b'#' && content[1] == b'!')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::NamedTempFile;

    #[test]
    fn test_has_shebang_true() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"#!/bin/bash\necho hello").unwrap();

        let result = has_shebang(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_has_shebang_false() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"echo hello").unwrap();

        let result = has_shebang(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_has_shebang_with_env() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"#!/usr/bin/env python\nprint('hello')").unwrap();

        let result = has_shebang(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_binary_file_not_flagged() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"\x7fELF\x02\x01\x01\x00").unwrap();

        // Binary files should return true (not flagged as missing shebang)
        let result = has_shebang(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_is_executable() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"#!/bin/bash\necho hello").unwrap();

        // Make file executable
        let mut perms = fs::metadata(file.path()).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(file.path(), perms).unwrap();

        let result = is_executable(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_not_executable() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"#!/bin/bash\necho hello").unwrap();

        // Ensure file is not executable
        let mut perms = fs::metadata(file.path()).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(file.path(), perms).unwrap();

        let result = is_executable(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"").unwrap();

        let result = has_shebang(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }
}
