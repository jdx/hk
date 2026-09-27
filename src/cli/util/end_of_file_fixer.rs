use super::text_files::{
    fix_in_order, for_each_in_order, read_rest_to_string, read_text_probe, regular_file_len,
};
use crate::Result;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Check for and optionally fix files to end with exactly one newline
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "write")]
pub struct EndOfFileFixer {
    /// Output a diff of the change. Cannot use with `fix`.
    #[usage(short, long, conflicts = "--fix")]
    pub diff: bool,

    /// Fix files to end with exactly one newline
    #[usage(short, long)]
    pub fix: bool,

    /// Files to check/fix
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,
}

impl EndOfFileFixer {
    pub async fn run(&self) -> Result<()> {
        if self.fix {
            // Fix mode always succeeds.
            return fix_in_order(&self.files, fixed_content);
        }

        let report = |path: &Path| -> Result<Option<String>> {
            if self.diff {
                generate_diff(path)
            } else {
                Ok(open_if_improper(path)?.map(|_| format!("{}\n", path.display())))
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

        // In check mode: exit with code 1 if issues found
        if found_issues {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Open a text file that doesn't end with exactly one newline, returning it
/// with the bytes read from its start so far. Returns `None` for files that
/// aren't text and files that already end properly, including empty files.
fn open_if_improper(path: &Path) -> Result<Option<(fs::File, Vec<u8>)>> {
    match regular_file_len(path) {
        Some(len) => open_if_improper_of_len(path, len),
        None => Ok(None),
    }
}

/// [`open_if_improper`] for a regular file of `len` bytes.
fn open_if_improper_of_len(path: &Path, len: u64) -> Result<Option<(fs::File, Vec<u8>)>> {
    if len == 0 {
        return Ok(None); // Empty files are text and already "correct"
    }
    let mut file = fs::File::open(path)?;
    let Some(head) = read_text_probe(&mut file, len)? else {
        return Ok(None);
    };
    let proper = if len == 1 {
        head[0] == b'\n'
    } else if head.len() as u64 == len {
        ends_properly(&head[head.len() - 2..])
    } else {
        // Read last 2 bytes to check for exactly one trailing newline
        let mut last_two = [0u8; 2];
        file.seek(SeekFrom::End(-2))?;
        file.read_exact(&mut last_two)?;
        file.seek(SeekFrom::Start(head.len() as u64))?;
        ends_properly(&last_two)
    };
    Ok((!proper).then_some((file, head)))
}

/// Whether a file's last two bytes are exactly one trailing newline.
fn ends_properly(last_two: &[u8]) -> bool {
    last_two[1] == b'\n' && last_two[0] != b'\n'
}

/// Normalize content to end with exactly one newline
fn normalize_ending(content: &str) -> String {
    let trimmed = content.trim_end_matches('\n');
    format!("{trimmed}\n")
}

/// The file's content, if it is a text file that doesn't end properly.
fn improper_content(opened: Option<(fs::File, Vec<u8>)>) -> Result<Option<String>> {
    match opened {
        Some((mut file, head)) => read_rest_to_string(&mut file, head).map(Some),
        None => Ok(None),
    }
}

/// The content of a regular file of `len` bytes ending with exactly one
/// newline, or `None` if it already does or isn't a text file.
fn fixed_content(path: &Path, len: u64) -> Result<Option<String>> {
    let content = improper_content(open_if_improper_of_len(path, len)?)?;
    Ok(content.map(|content| normalize_ending(&content)))
}

/// Generate a unified diff showing the fix
/// Returns None if file already has proper ending
fn generate_diff(path: &Path) -> Result<Option<String>> {
    let Some(original) = improper_content(open_if_improper(path)?)? else {
        return Ok(None);
    };
    let fixed = normalize_ending(&original);
    let path_str = path.display().to_string();
    let diff = crate::diff::render_unified_diff(
        &original,
        &fixed,
        &format!("a/{}", path_str),
        &format!("b/{}", path_str),
    );

    Ok(Some(diff))
}

/// Check if a text file has a proper ending (exactly one trailing newline)
#[cfg(test)]
fn has_proper_ending(path: &Path) -> Result<bool> {
    Ok(open_if_improper(path)?.is_none())
}

/// Fix a file to end with exactly one newline
#[cfg(test)]
fn fix_end_of_file(path: &Path) -> Result<()> {
    let Some(len) = regular_file_len(path) else {
        return Ok(());
    };
    if let Some(fixed) = fixed_content(path, len)? {
        fs::write(path, fixed)?;
    }
    Ok(())
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
    fn test_has_proper_ending_true() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "line1").unwrap();
        writeln!(file, "line2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(has_proper_ending(&path).unwrap());
    }

    #[test]
    fn test_has_proper_ending_missing_newline() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "line1\nline2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_proper_ending(&path).unwrap());
    }

    #[test]
    fn test_has_proper_ending_extra_newlines() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "line1\nline2\n\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_proper_ending(&path).unwrap());
    }

    #[test]
    fn test_has_proper_ending_single_newline_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file).unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        // A file containing only \n has len==1, which is handled as a special case
        assert!(has_proper_ending(&path).unwrap());
    }

    #[test]
    fn test_fix_end_of_file() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "line1\nline2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should not have final newline
        assert!(!has_proper_ending(&path).unwrap());

        // Fix it
        fix_end_of_file(&path).unwrap();

        // Should now have final newline
        assert!(has_proper_ending(&path).unwrap());

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

        // Should already have proper ending
        assert!(has_proper_ending(&path).unwrap());

        let content_before = fs::read_to_string(&path).unwrap();

        // Fix should do nothing
        fix_end_of_file(&path).unwrap();

        let content_after = fs::read_to_string(&path).unwrap();
        assert_eq!(content_before, content_after);
    }

    #[test]
    fn test_fix_extra_trailing_newlines() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "line1\nline2\n\n\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();

        // Should detect extra trailing newlines
        assert!(!has_proper_ending(&path).unwrap());

        // Fix it
        fix_end_of_file(&path).unwrap();

        // Should now have proper ending
        assert!(has_proper_ending(&path).unwrap());

        // Verify content
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "line1\nline2\n");
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        // Empty file is considered correct
        assert!(has_proper_ending(&path).unwrap());

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
