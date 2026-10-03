use super::git_exec_bit::executable_flags;
use crate::Result;
use std::fs;
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

        let mut any_tracked = false;
        let mut any_untracked = false;
        let flags = executable_flags(&self.files)?;
        for (file_path, flag) in self.files.iter().zip(flags) {
            if flag.executable == Some(false) && has_shebang(file_path)? {
                if flag.tracked {
                    any_tracked = true;
                } else {
                    any_untracked = true;
                }
                println!(
                    "{}: has a shebang but is not marked executable",
                    file_path.display()
                );
                found_issues = true;
            }
        }

        if found_issues {
            println!();
            println!("If it is supposed to be executable:");
            if any_tracked {
                println!(
                    "  - tracked files: run `git update-index --chmod=+x <file>` (git records the mode, not the filesystem)"
                );
            }
            if any_untracked {
                println!(
                    "  - untracked files: run `chmod +x <file>`, then `git add <file>` (or `git add` first, then `git update-index --chmod=+x <file>`)"
                );
            }
            println!("If not, remove the shebang.");
            return Err(eyre::eyre!("Non-executable files with shebangs found"));
        }

        Ok(())
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
}
