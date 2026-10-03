use super::text_files::{
    fix_in_order, for_each_in_order, read_rest_to_string, read_text_probe, regular_file_len,
};
use crate::Result;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Check for and optionally fix trailing whitespace
///
/// Only spaces and tabs before a line's terminator count as trailing
/// whitespace. Each line keeps its own terminator, so a CRLF file stays CRLF.
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "write")]
pub struct TrailingWhitespace {
    /// Output a diff of the change. Cannot use with `fix`.
    #[usage(short, long, conflicts = "--fix")]
    pub diff: bool,

    /// Fix trailing whitespace by removing the spaces and tabs
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
    let Some(len) = regular_file_len(path) else {
        return Ok(None);
    };
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

/// Split `content` into lines as `(body, terminator)`. The terminator is
/// `"\r\n"` or `"\n"`, or empty for a last line without one. A lone `\r` is
/// not a terminator (hk doesn't target legacy Mac line endings), so it stays
/// part of the body.
fn lines(content: &str) -> impl Iterator<Item = (&str, &str)> {
    content.split_inclusive('\n').map(|line| {
        if let Some(body) = line.strip_suffix("\r\n") {
            (body, "\r\n")
        } else if let Some(body) = line.strip_suffix('\n') {
            (body, "\n")
        } else {
            (line, "")
        }
    })
}

/// Check if a file has trailing whitespace
fn has_trailing_whitespace(path: &Path) -> Result<bool> {
    let Some(content) = read_text(path)? else {
        return Ok(false);
    };
    Ok(lines(&content).any(|(body, _)| body.ends_with([' ', '\t'])))
}

/// Strip spaces and tabs before each line's terminator, keeping the terminator
fn strip_trailing_whitespace(original: &str) -> String {
    let mut fixed = String::with_capacity(original.len());
    for (body, terminator) in lines(original) {
        fixed.push_str(body.trim_end_matches([' ', '\t']));
        fixed.push_str(terminator);
    }
    fixed
}

/// The file's content without trailing whitespace, or `None` if it has none
/// or isn't a text file.
fn fixed_content(path: &Path) -> Result<Option<String>> {
    let Some(original) = read_text(path)? else {
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
    let Some(fixed) = fixed_content(path)? else {
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
    fn test_clean_crlf_has_no_trailing_whitespace() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello\r\nworld\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_trailing_whitespace(&path).unwrap());
    }

    #[test]
    fn test_has_trailing_whitespace_crlf_with_spaces() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello  \r\nworld\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(has_trailing_whitespace(&path).unwrap());
    }

    #[test]
    fn test_fix_trailing_whitespace_crlf_keeps_crlf() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello  \r\nworld\t\r\nclean\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(fix_trailing_whitespace(&path).unwrap());
        assert!(!has_trailing_whitespace(&path).unwrap());
        assert_eq!(fs::read(&path).unwrap(), b"hello\r\nworld\r\nclean\r\n");
    }

    #[test]
    fn test_clean_crlf_file_is_not_modified() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello\r\nworld\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!fix_trailing_whitespace(&path).unwrap());
        assert_eq!(fs::read(&path).unwrap(), b"hello\r\nworld\r\n");
    }

    #[test]
    fn test_fix_keeps_each_lines_terminator() {
        assert_eq!(
            strip_trailing_whitespace("a  \r\nb \nc\t\r\n\r\n  \n"),
            "a\r\nb\nc\r\n\r\n\n"
        );
    }

    #[test]
    fn test_fix_strips_only_spaces_and_tabs() {
        // A form feed or a lone carriage return is not trailing whitespace.
        assert_eq!(strip_trailing_whitespace("a\x0c\nb\r"), "a\x0c\nb\r");
        assert_eq!(strip_trailing_whitespace("a \rb \n"), "a \rb\n");
    }

    #[test]
    fn test_fix_crlf_without_final_newline() {
        assert_eq!(strip_trailing_whitespace("a \r\nb  "), "a\r\nb");
    }
}
