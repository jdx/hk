use crate::Result;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Check for and optionally fix missing final newlines in files
#[derive(Debug, clap::Args)]
pub struct EndOfFileFixer {
    /// Fix files by adding final newline
    #[clap(short, long, conflicts_with = "diff")]
    pub fix: bool,

    /// Output a diff of the change. Cannot use with `fix`.
    #[clap(short, long)]
    pub diff: bool,

    /// Files to check/fix
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl EndOfFileFixer {
    pub async fn run(&self) -> Result<()> {
        let mut found_issues = false;

        for file_path in &self.files {
            // Skip non-text files
            if !is_text_file(file_path)? {
                continue;
            }

            if self.fix {
                fix_end_of_file(file_path)?;
            } else if self.diff {
                if let Some(diff) = generate_diff(file_path)? {
                    print!("{}", diff);
                    found_issues = true;
                }
            } else if !has_final_newline(file_path)? {
                println!("{}", file_path.display());
                found_issues = true;
            }
        }

        // In check mode: exit with code 1 if issues found
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
        return Ok(true); // Empty files are text and already "correct"
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

/// Generate a unified diff showing the addition of a final newline
/// Returns None if file already has final newline
fn generate_diff(path: &PathBuf) -> Result<Option<String>> {
    if has_final_newline(path)? {
        return Ok(None);
    }

    let original = fs::read_to_string(path)?;
    let fixed = format!("{}\n", original);
    let path_str = path.display().to_string();
    let diff = crate::diff::render_unified_diff(
        &original,
        &fixed,
        &format!("a/{}", path_str),
        &format!("b/{}", path_str),
    );

    Ok(Some(diff))
}

/// Check if a file has a final newline
fn has_final_newline(path: &PathBuf) -> Result<bool> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Ok(true); // Empty files are considered correct
    }

    // Read only the last byte
    let mut file = fs::File::open(path)?;
    let mut last_byte = [0u8; 1];

    // Seek to last byte and read it
    use std::io::Seek;
    file.seek(std::io::SeekFrom::End(-1))?;
    file.read_exact(&mut last_byte)?;

    Ok(last_byte[0] == b'\n')
}

/// Add final newline to a file if missing
fn fix_end_of_file(path: &PathBuf) -> Result<()> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Ok(()); // Empty files don't need fixing
    }

    if !has_final_newline(path)? {
        // Open file in append mode and add newline
        let mut file = fs::OpenOptions::new().append(true).open(path)?;
        writeln!(file)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_has_final_newline_true() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(has_final_newline(&path).unwrap());
    }

    #[test]
    fn test_has_final_newline_false() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "line1\nline2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_final_newline(&path).unwrap());
    }

    #[test]
    fn test_fix_end_of_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "line1\nline2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should not have final newline
        assert!(!has_final_newline(&path).unwrap());

        // Fix it
        fix_end_of_file(&path).unwrap();

        // Should now have final newline
        assert!(has_final_newline(&path).unwrap());

        // Verify content
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "line1\nline2\n");
    }

    #[test]
    fn test_fix_already_correct() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should already have final newline
        assert!(has_final_newline(&path).unwrap());

        let content_before = fs::read_to_string(&path).unwrap();

        // Fix should do nothing
        fix_end_of_file(&path).unwrap();

        let content_after = fs::read_to_string(&path).unwrap();
        assert_eq!(content_before, content_after);
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        // Empty file is considered correct
        assert!(has_final_newline(&path).unwrap());

        // Fix should do nothing
        fix_end_of_file(&path).unwrap();

        // Still empty
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "");
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
        file.write_all(&[0x00, 0x01, 0x02, 0x03, 0xFF]).unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!is_text_file(&path).unwrap());
    }
}
