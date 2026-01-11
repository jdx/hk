use crate::Result;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct MixedLineEnding {
    /// Output a diff of the change. Cannot use with `fix`.
    #[clap(short, long, conflicts_with = "fix")]
    pub diff: bool,

    /// Fix mixed line endings by normalizing to LF
    #[clap(short, long)]
    pub fix: bool,

    /// Files to check or fix
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl MixedLineEnding {
    pub async fn run(&self) -> Result<()> {
        let mut found_mixed = false;

        for file_path in &self.files {
            if self.fix {
                if has_mixed_line_endings(file_path)? {
                    fix_line_endings(file_path)?;
                }
            } else if self.diff {
                if let Some(diff) = generate_diff(file_path)? {
                    print!("{}", diff);
                    found_mixed = true;
                }
            } else if has_mixed_line_endings(file_path)? {
                println!("{}", file_path.display());
                found_mixed = true;
            }
        }

        if !self.fix && found_mixed {
            std::process::exit(1);
        }

        Ok(())
    }
}

fn generate_diff(path: &PathBuf) -> Result<Option<String>> {
    if !has_mixed_line_endings(path)? {
        return Ok(None);
    }

    let original = fs::read_to_string(path)?;
    let fixed = original.replace("\r\n", "\n");
    let path_str = path.display().to_string();
    let diff = crate::diff::render_unified_diff(
        &original,
        &fixed,
        &format!("a/{}", path_str),
        &format!("b/{}", path_str),
    );

    Ok(Some(diff))
}

fn has_mixed_line_endings(path: &PathBuf) -> Result<bool> {
    // Skip directories
    if path.is_dir() {
        return Ok(false);
    }

    let content = fs::read(path)?;

    // Skip binary files
    if content.contains(&0) {
        return Ok(false);
    }

    let mut found_lf = false;
    let mut found_crlf = false;

    let mut i = 0;
    while i < content.len() {
        if content[i] == b'\n' {
            // Check if preceded by \r
            if i > 0 && content[i - 1] == b'\r' {
                found_crlf = true;
            } else {
                found_lf = true;
            }
        }
        i += 1;
    }

    Ok(found_lf && found_crlf)
}

fn fix_line_endings(path: &PathBuf) -> Result<()> {
    // Skip directories
    if path.is_dir() {
        return Ok(());
    }

    let content = fs::read(path)?;

    // Convert all CRLF to LF
    let mut normalized = Vec::new();
    let mut i = 0;
    while i < content.len() {
        if i + 1 < content.len() && content[i] == b'\r' && content[i + 1] == b'\n' {
            // Skip the \r, keep only \n
            normalized.push(b'\n');
            i += 2;
        } else {
            normalized.push(content[i]);
            i += 1;
        }
    }

    fs::write(path, normalized)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_no_mixed_endings_lf_only() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"line1\nline2\nline3\n").unwrap();

        let result = has_mixed_line_endings(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_no_mixed_endings_crlf_only() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"line1\r\nline2\r\nline3\r\n").unwrap();

        let result = has_mixed_line_endings(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_mixed_endings() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"line1\r\nline2\nline3\r\n").unwrap();

        let result = has_mixed_line_endings(&file.path().to_path_buf()).unwrap();
        assert!(result);
    }

    #[test]
    fn test_fix_mixed_endings() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"line1\r\nline2\nline3\r\n").unwrap();

        fix_line_endings(&file.path().to_path_buf()).unwrap();

        let content = fs::read(file.path()).unwrap();
        assert_eq!(content, b"line1\nline2\nline3\n");
    }

    #[test]
    fn test_binary_file_skipped() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"binary\x00data\r\nwith\nlines").unwrap();

        let result = has_mixed_line_endings(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_file_with_no_line_endings() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"just one line").unwrap();

        let result = has_mixed_line_endings(&file.path().to_path_buf()).unwrap();
        assert!(!result);
    }
}
