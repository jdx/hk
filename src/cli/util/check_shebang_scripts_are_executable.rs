use crate::Result;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct CheckShebangScriptsAreExecutable {
    /// Files to check
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,
}

impl CheckShebangScriptsAreExecutable {
    pub async fn run(&self) -> Result<()> {
        let mut found_issues = false;

        for file_path in &self.files {
            if has_shebang(file_path)? && !is_executable(file_path)? {
                println!(
                    "{}: has a shebang but is not marked executable",
                    file_path.display()
                );
                found_issues = true;
            }
        }

        if found_issues {
            println!();
            println!("If it is supposed to be executable, run `chmod +x <file>`.");
            println!("If not, remove the shebang.");
            return Err(eyre::eyre!("Non-executable files with shebangs found"));
        }

        Ok(())
    }
}

fn is_executable(path: &PathBuf) -> Result<bool> {
    let metadata = fs::metadata(path)?;

    if metadata.is_dir() {
        return Ok(true);
    }

    #[cfg(unix)]
    {
        Ok(metadata.permissions().mode() & 0o111 != 0)
    }

    #[cfg(not(unix))]
    {
        Ok(true)
    }
}

fn has_shebang(path: &PathBuf) -> Result<bool> {
    let content = fs::read(path)?;

    Ok(content.starts_with(b"#!"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::NamedTempFile;

    #[test]
    fn test_has_shebang_true() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"#!/bin/bash\necho hello").unwrap();

        assert!(has_shebang(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    fn test_has_shebang_false() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"echo hello").unwrap();

        assert!(!has_shebang(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    fn test_has_shebang_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"").unwrap();

        assert!(!has_shebang(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    fn test_has_shebang_ignores_binary_content() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"\x7fELF\x02\x01\x01\x00").unwrap();

        assert!(!has_shebang(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    #[cfg(unix)]
    fn test_is_executable() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"#!/bin/bash\necho hello").unwrap();

        let mut perms = fs::metadata(file.path()).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(file.path(), perms).unwrap();

        assert!(is_executable(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    #[cfg(unix)]
    fn test_not_executable() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"#!/bin/bash\necho hello").unwrap();

        let mut perms = fs::metadata(file.path()).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(file.path(), perms).unwrap();

        assert!(!is_executable(&file.path().to_path_buf()).unwrap());
    }
}
