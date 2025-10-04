use crate::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct PythonDebugStatements {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl PythonDebugStatements {
    pub async fn run(&self) -> Result<()> {
        let mut found_debug = false;

        for file_path in &self.files {
            if has_debug_statements(file_path)? {
                println!("{}", file_path.display());
                found_debug = true;
            }
        }

        if found_debug {
            return Err(eyre::eyre!("Debug statements found in Python files"));
        }

        Ok(())
    }
}

fn has_debug_statements(path: &PathBuf) -> Result<bool> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(false), // File doesn't exist or can't be read
    };

    // Common Python debug patterns
    let debug_patterns = [
        "import pdb",
        "import ipdb",
        "import pudb",
        "import pdbpp",
        "pdb.set_trace(",
        "ipdb.set_trace(",
        "pudb.set_trace(",
        "breakpoint(",
        "from pdb import",
        "from ipdb import",
        "from pudb import",
    ];

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Check for debug patterns
        for pattern in &debug_patterns {
            if trimmed.contains(pattern) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_import_pdb() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
import pdb
pdb.set_trace()
"#,
        )
        .unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_import_ipdb() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
import ipdb
ipdb.set_trace()
"#,
        )
        .unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_breakpoint() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
def debug_me():
    breakpoint()
    print("After breakpoint")
"#,
        )
        .unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_clean_code() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
def hello():
    print("Hello, world!")
"#,
        )
        .unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_commented_debug() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
def hello():
    # import pdb; pdb.set_trace()
    print("Hello")
"#,
        )
        .unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(!result); // Commented out, should not be detected
    }

    #[test]
    fn test_from_pdb_import() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
from pdb import set_trace
set_trace()
"#,
        )
        .unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "").unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_pudb() {
        let file = NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
import pudb
pudb.set_trace()
"#,
        )
        .unwrap();

        let result = has_debug_statements(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_nonexistent_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("nonexistent");

        let result = has_debug_statements(&file).unwrap();
        assert!(!result);
    }
}
