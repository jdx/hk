use super::text_files::{
    fix_in_order, for_each_in_order, read_rest_to_string, read_text_probe, regular_file_len,
};
use crate::Result;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Check for and optionally fix missing final newlines
///
/// A missing final newline is added as the file's most frequent line ending,
/// so a CRLF file gets CRLF. Every line ending in the file counts, as in
/// `mixed-line-ending`, and a tie gives LF. Trailing blank lines, LF or CRLF,
/// are removed. A file that already ends with exactly one newline is left as
/// it is.
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
    let Some(len) = regular_file_len(path) else {
        return Ok(None);
    };
    if len == 0 {
        return Ok(None); // Empty files are text and already "correct"
    }
    let mut file = fs::File::open(path)?;
    let Some(head) = read_text_probe(&mut file, len)? else {
        return Ok(None);
    };
    // The last four bytes are enough to see whether the file ends with
    // exactly one terminator, whether `\n` or `\r\n`.
    let tail = if head.len() as u64 == len {
        head[head.len().saturating_sub(4)..].to_vec()
    } else {
        let mut tail = [0u8; 4];
        file.seek(SeekFrom::End(-4))?;
        file.read_exact(&mut tail)?;
        file.seek(SeekFrom::Start(head.len() as u64))?;
        tail.to_vec()
    };
    let proper = ends_properly(&tail);
    Ok((!proper).then_some((file, head)))
}

/// Whether a file's last bytes (up to four) are exactly one terminator: a
/// final `\n` or `\r\n` that no blank line or stray `\r` precedes.
fn ends_properly(tail: &[u8]) -> bool {
    let Some((&b'\n', before)) = tail.split_last() else {
        return false;
    };
    // A CRLF terminator's `\r` isn't part of the preceding line's content.
    let before = before.strip_suffix(b"\r").unwrap_or(before);
    !matches!(before.last(), Some(b'\n' | b'\r'))
}

/// The line ending most of the file's lines use, `"\r\n"` or `"\n"`, counting
/// every line ending in the file, as `mixed-line-ending` does. A tie, or a file
/// with no line ending, gives `"\n"`, so the two agree on which ending a mixed
/// file is normalized to.
fn dominant_terminator(content: &str) -> &'static str {
    let lf = content.matches('\n').count();
    let crlf = content.matches("\r\n").count();
    if crlf > lf - crlf { "\r\n" } else { "\n" }
}

/// Normalize content to end with exactly one terminator, the file's dominant
/// one. Trailing blank lines, whether LF or CRLF, and a stray `\r` are removed.
fn normalize_ending(content: &str) -> String {
    let body = content.trim_end_matches(['\r', '\n']);
    format!("{body}{}", dominant_terminator(content))
}

/// The file's content, if it is a text file that doesn't end properly.
fn improper_content(path: &Path) -> Result<Option<String>> {
    match open_if_improper(path)? {
        Some((mut file, head)) => read_rest_to_string(&mut file, head).map(Some),
        None => Ok(None),
    }
}

/// The file's content ending with exactly one newline, or `None` if it
/// already does or isn't a text file.
fn fixed_content(path: &Path) -> Result<Option<String>> {
    Ok(improper_content(path)?.map(|content| normalize_ending(&content)))
}

/// Generate a unified diff showing the fix
/// Returns None if file already has proper ending
fn generate_diff(path: &Path) -> Result<Option<String>> {
    let Some(original) = improper_content(path)? else {
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
    if let Some(fixed) = fixed_content(path)? {
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
    fn test_crlf_file_with_final_crlf_is_proper() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"line1\r\nline2\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(has_proper_ending(&path).unwrap());
        fix_end_of_file(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"line1\r\nline2\r\n");
    }

    #[test]
    fn test_fix_missing_newline_appends_crlf_to_crlf_file() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"line1\r\nline2").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_proper_ending(&path).unwrap());
        fix_end_of_file(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"line1\r\nline2\r\n");
        assert!(has_proper_ending(&path).unwrap());
    }

    #[test]
    fn test_fix_trailing_blank_crlf_lines() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"line1\r\nline2\r\n\r\n\r\n").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_proper_ending(&path).unwrap());
        fix_end_of_file(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"line1\r\nline2\r\n");
        assert!(has_proper_ending(&path).unwrap());
    }

    #[test]
    fn test_normalize_ending_uses_dominant_terminator() {
        assert_eq!(normalize_ending("a\nb\nc\r\nd"), "a\nb\nc\r\nd\n");
        assert_eq!(normalize_ending("a\r\nb\r\nc\nd"), "a\r\nb\r\nc\nd\r\n");
        // A tie, and a file with no line ending, get LF.
        assert_eq!(normalize_ending("a\r\nb\nc"), "a\r\nb\nc\n");
        assert_eq!(normalize_ending("a"), "a\n");
    }

    #[test]
    fn test_normalize_ending_counts_every_line_ending_like_mixed_line_ending() {
        // A tie goes to LF, even when the last line is CRLF.
        assert_eq!(normalize_ending("a\nb\r\nc"), "a\nb\r\nc\n");
        assert_eq!(normalize_ending("a\r\nb\nc"), "a\r\nb\nc\n");
        // The blank lines being removed still count toward the majority.
        assert_eq!(normalize_ending("a\r\nb\r\n\n\n\n\n"), "a\r\nb\n");
        assert_eq!(normalize_ending("a\nb\n\r\n\r\n\r\n\r\n"), "a\nb\r\n");
    }

    #[test]
    fn test_normalize_ending_is_proper_and_stable() {
        for content in ["a\r\nb\r\n\n\n\n\n", "a\nb\r\nc"] {
            let fixed = normalize_ending(content);
            assert_eq!(normalize_ending(&fixed), fixed);
            assert!(ends_properly(
                &fixed.as_bytes()[fixed.len().saturating_sub(4)..]
            ));
        }
    }

    #[test]
    fn test_normalize_ending_mixed_blank_lines() {
        assert_eq!(normalize_ending("a\r\nb\r\n\n\r\n"), "a\r\nb\r\n");
        assert_eq!(normalize_ending("a\nb\n\r\n"), "a\nb\n");
        assert_eq!(normalize_ending("a\r\nb\r\n\r"), "a\r\nb\r\n");
    }

    #[test]
    fn test_ends_properly() {
        assert!(ends_properly(b"a\n"));
        assert!(ends_properly(b"a\r\n"));
        assert!(ends_properly(b"\n"));
        assert!(ends_properly(b"\r\n"));
        assert!(!ends_properly(b"a"));
        assert!(!ends_properly(b"a\r"));
        assert!(!ends_properly(b"a\n\n"));
        assert!(!ends_properly(b"a\r\n\r\n"));
        assert!(!ends_properly(b"a\n\r\n"));
        assert!(!ends_properly(b"a\r\n\n"));
        assert!(!ends_properly(b"a\r\r\n"));
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
