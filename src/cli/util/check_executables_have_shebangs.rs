use super::git_exec_bit::executable_flags;
use crate::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct CheckExecutablesHaveShebangs {
    /// Files to check
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,
}

impl CheckExecutablesHaveShebangs {
    pub async fn run(&self) -> Result<()> {
        let mut found_issues = false;

        let flags = executable_flags(&self.files)?;
        for (file_path, flag) in self.files.iter().zip(flags) {
            if flag.executable == Some(true) && !has_shebang(file_path)? {
                println!("{}", file_path.display());
                found_issues = true;
            }
        }

        if found_issues {
            return Err(eyre::eyre!("Executable files without shebangs found"));
        }

        Ok(())
    }
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
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"").unwrap();

        let result = has_shebang(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }
}
