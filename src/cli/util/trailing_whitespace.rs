use crate::Result;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

/// Check for and optionally fix trailing whitespace in files
#[derive(Debug, clap::Args)]
pub struct TrailingWhitespace {
    /// Output a diff of the change. Cannot use with `fix`.
    #[clap(short, long, conflicts_with = "fix")]
    pub diff: bool,

    /// Fix trailing whitespace by removing it
    #[clap(short, long)]
    pub fix: bool,

    /// Files to check/fix
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl TrailingWhitespace {
    pub async fn run(&self) -> Result<()> {
        let mut found_issues = false;

        for file_path in &self.files {
            // Skip non-text files
            if !is_text_file(file_path)? {
                continue;
            }

            if self.fix {
                fix_trailing_whitespace(file_path)?;
            } else if self.diff {
                if let Some(diff) = generate_diff(file_path)? {
                    print!("{}", diff);
                    found_issues = true;
                }
            } else if has_trailing_whitespace(file_path)? {
                println!("{}", file_path.display());
                found_issues = true;
            }
        }

        // In check/diff mode: exit with code 1 if issues found
        // Fix mode always succeeds
        if !self.fix && found_issues {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Check if a file is a text file
/// Uses a heuristic: reads the first 8KB and checks if it's valid UTF-8
fn is_text_file(path: &PathBuf) -> Result<bool> {
    if !path.exists() || !path.is_file() {
        return Ok(false);
    }

    // Check if file is empty
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Ok(true); // Empty files are text
    }

    // Read first 8KB to detect if it's text
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0; 8192.min(metadata.len() as usize)];
    file.read_exact(&mut buffer)?;

    // Check for null bytes (common in binary files)
    if buffer.contains(&0) {
        return Ok(false);
    }

    // Try to validate as UTF-8
    Ok(std::str::from_utf8(&buffer).is_ok())
}

/// Check if a file has trailing whitespace
fn has_trailing_whitespace(path: &PathBuf) -> Result<bool> {
    let content = fs::read_to_string(path)?;

    for line in content.split('\n') {
        // Check for whitespace (including \r) before the newline
        if line != line.trim_end() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Generate a unified diff showing trailing whitespace removal
/// Returns None if no changes needed
fn generate_diff(path: &PathBuf) -> Result<Option<String>> {
    let original = fs::read_to_string(path)?;
    let fixed: String = original
        .split_inclusive('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        + if original.ends_with('\n') { "\n" } else { "" };

    if original == fixed {
        return Ok(None);
    }

    let path_str = path.display().to_string();
    let diff = crate::diff::render_unified_diff(
        &original,
        &fixed,
        &format!("a/{}", path_str),
        &format!("b/{}", path_str),
    );

    Ok(Some(diff))
}

/// Fix trailing whitespace in a file, returns true if file was modified
fn fix_trailing_whitespace(path: &PathBuf) -> Result<bool> {
    let original = fs::read_to_string(path)?;
    let fixed: String = original
        .split_inclusive('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        + if original.ends_with('\n') { "\n" } else { "" };

    if original == fixed {
        return Ok(false);
    }

    fs::write(path, &fixed)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_has_trailing_whitespace() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "no trailing").unwrap();
        writeln!(file, "has trailing  ").unwrap();

        let path = file.path().to_path_buf();
        assert!(has_trailing_whitespace(&path).unwrap());
    }

    #[test]
    fn test_no_trailing_whitespace() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "no trailing").unwrap();
        writeln!(file, "also clean").unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_trailing_whitespace(&path).unwrap());
    }

    #[test]
    fn test_fix_trailing_whitespace() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "clean line").unwrap();
        writeln!(file, "trailing  ").unwrap();
        writeln!(file, "more trailing\t").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should detect and fix
        assert!(fix_trailing_whitespace(&path).unwrap());

        // Should be clean now
        assert!(!has_trailing_whitespace(&path).unwrap());

        // Verify content
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "clean line\ntrailing\nmore trailing\n");
    }

    #[test]
    fn test_fix_already_clean() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "clean line").unwrap();
        writeln!(file, "also clean").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should not modify
        assert!(!fix_trailing_whitespace(&path).unwrap());
    }

    #[test]
    fn test_is_text_file_with_text() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "This is a text file").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(is_text_file(&path).unwrap());
    }

    #[test]
    fn test_is_text_file_with_binary() {
        let mut file = NamedTempFile::new().unwrap();
        // Write binary data with null bytes
        file.write_all(&[0x00, 0x01, 0x02, 0x03, 0xFF]).unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!is_text_file(&path).unwrap());
    }

    #[test]
    fn test_is_text_file_with_empty() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();
        assert!(is_text_file(&path).unwrap()); // Empty files are considered text
    }

    #[test]
    fn test_fix_preserves_no_final_newline() {
        let mut file = NamedTempFile::new().unwrap();
        // Write content without final newline
        write!(file, "line1  \nline2\t\nline3").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should fix trailing whitespace
        assert!(fix_trailing_whitespace(&path).unwrap());

        // Verify no final newline was added
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "line1\nline2\nline3");
        assert!(!content.ends_with('\n'));
    }

    #[test]
    fn test_fix_preserves_final_newline() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line1  ").unwrap();
        writeln!(file, "line2\t").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should fix trailing whitespace
        assert!(fix_trailing_whitespace(&path).unwrap());

        // Verify final newline was preserved
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "line1\nline2\n");
        assert!(content.ends_with('\n'));
    }

    #[test]
    fn test_has_trailing_whitespace_crlf() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello\r\nworld\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(has_trailing_whitespace(&path).unwrap());
    }

    #[test]
    fn test_fix_trailing_whitespace_crlf() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello\r\nworld\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should detect and fix
        assert!(fix_trailing_whitespace(&path).unwrap());

        // Should be clean now
        assert!(!has_trailing_whitespace(&path).unwrap());

        // Verify \r is stripped
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello\nworld\n");
    }
}
