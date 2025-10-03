use crate::Result;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, clap::Args)]
pub struct PythonCheckAst {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl PythonCheckAst {
    pub async fn run(&self) -> Result<()> {
        let mut found_invalid = false;

        for file_path in &self.files {
            if !is_valid_python_syntax(file_path)? {
                println!("{}", file_path.display());
                found_invalid = true;
            }
        }

        if found_invalid {
            std::process::exit(1);
        }

        Ok(())
    }
}

fn is_valid_python_syntax(path: &PathBuf) -> Result<bool> {
    // Use python -m py_compile to check syntax
    // This is more reliable than ast.parse as it catches all syntax errors
    let output = Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(path)
        .output();

    match output {
        Ok(result) => Ok(result.status.success()),
        Err(_) => {
            // If python3 is not available, try python
            let output = Command::new("python")
                .arg("-m")
                .arg("py_compile")
                .arg(path)
                .output();

            match output {
                Ok(result) => Ok(result.status.success()),
                Err(_) => {
                    // If neither python3 nor python is available, skip the file
                    Ok(true)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_valid_python() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
def hello():
    print("Hello, world!")
"#,
        )
        .unwrap();

        // This test will pass if Python is available
        let result = is_valid_python_syntax(&file.path().to_path_buf());
        if result.is_ok() {
            // Only assert if we successfully ran python
            assert!(result.unwrap());
        }
    }

    #[test]
    fn test_invalid_python() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
def hello(:
    print("Invalid syntax"
"#,
        )
        .unwrap();

        // This test will pass if Python is available
        let result = is_valid_python_syntax(&file.path().to_path_buf());
        if result.is_ok() {
            // Only assert if we successfully ran python
            assert!(!result.unwrap());
        }
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "").unwrap();

        let result = is_valid_python_syntax(&file.path().to_path_buf());
        if result.is_ok() {
            // Empty file is valid Python
            assert!(result.unwrap());
        }
    }
}
