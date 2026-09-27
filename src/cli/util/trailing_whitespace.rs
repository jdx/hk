use super::text_files::{
    fix_in_order, for_each_in_order, read_rest_to_string, read_text_probe, regular_file_len,
};
use crate::Result;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Check for and optionally fix trailing whitespace in files
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "write")]
pub struct TrailingWhitespace {
    /// Output a diff of the change. Cannot use with `fix`.
    #[usage(short, long, conflicts = "--fix")]
    pub diff: bool,

    /// Fix trailing whitespace by removing it
    #[usage(short, long)]
    pub fix: bool,

    /// Files to check/fix
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,
}

impl TrailingWhitespace {
    pub async fn run(&self) -> Result<()> {
        if self.fix {
            // Fix mode always succeeds.
            return fix_in_order(&self.files, fixed_content);
        }

        let report = |path: &Path| -> Result<Option<String>> {
            if self.diff {
                generate_diff(path)
            } else {
                Ok(has_trailing_whitespace(path)?.then(|| format!("{}\n", path.display())))
            }
        };
        let mut out = io::BufWriter::new(io::stdout().lock());
        let mut found_issues = false;
        let reported = for_each_in_order(
            &self.files,
            |path| report(path),
            |_, report| {
                if let Some(report) = report? {
                    out.write_all(report.as_bytes())?;
                    found_issues = true;
                }
                Ok(())
            },
        );
        out.flush()?;
        reported?;

        // In check/diff mode: exit with code 1 if issues found
        if found_issues {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Read a file's content, or `None` for a file that isn't text.
fn read_text(path: &Path) -> Result<Option<String>> {
    match regular_file_len(path) {
        Some(len) => read_text_of_len(path, len),
        None => Ok(None),
    }
}

/// Read the content of a regular file of `len` bytes, or `None` if it isn't
/// text.
fn read_text_of_len(path: &Path, len: u64) -> Result<Option<String>> {
    let mut file = fs::File::open(path)?;
    let head = if len == 0 {
        Vec::new() // Empty files are text
    } else {
        match read_text_probe(&mut file, len)? {
            Some(head) => head,
            None => return Ok(None),
        }
    };
    read_rest_to_string(&mut file, head).map(Some)
}

/// Check if a file has trailing whitespace
fn has_trailing_whitespace(path: &Path) -> Result<bool> {
    let Some(content) = read_text(path)? else {
        return Ok(false);
    };

    for line in content.split('\n') {
        // Check for whitespace (including \r) before the newline
        if line != line.trim_end() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Strip trailing whitespace from each line in the content
fn strip_trailing_whitespace(original: &str) -> String {
    original
        .split_inclusive('\n')
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        + if original.ends_with('\n') { "\n" } else { "" }
}

/// The content of a regular file of `len` bytes without trailing
/// whitespace, or `None` if it has none or isn't a text file.
fn fixed_content(path: &Path, len: u64) -> Result<Option<String>> {
    let Some(original) = read_text_of_len(path, len)? else {
        return Ok(None);
    };
    let fixed = strip_trailing_whitespace(&original);
    Ok((original != fixed).then_some(fixed))
}

/// Generate a unified diff showing trailing whitespace removal
/// Returns None if no changes needed
fn generate_diff(path: &Path) -> Result<Option<String>> {
    let Some(original) = read_text(path)? else {
        return Ok(None);
    };
    let fixed = strip_trailing_whitespace(&original);

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
#[cfg(test)]
fn fix_trailing_whitespace(path: &Path) -> Result<bool> {
    let Some(len) = regular_file_len(path) else {
        return Ok(false);
    };
    let Some(fixed) = fixed_content(path, len)? else {
        return Ok(false);
    };
    fs::write(path, &fixed)?;
    Ok(true)
}

/// Check if a file is a text file
#[cfg(test)]
fn is_text_file(path: &Path) -> Result<bool> {
    let Some(len) = regular_file_len(path) else {
        return Ok(false);
    };
    if len == 0 {
        return Ok(true); // Empty files are text
    }
    Ok(read_text_probe(&mut fs::File::open(path)?, len)?.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
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
