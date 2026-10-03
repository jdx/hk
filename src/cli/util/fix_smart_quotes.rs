use crate::Result;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

// https://en.wikipedia.org/wiki/Quotation_mark#Unicode_code_point_table
const UTF8_DOUBLE_QUOTE_CODEPOINTS: &[char] = &[
    '\u{FF02}', // FULLWIDTH QUOTATION MARK
    '\u{201C}', // LEFT DOUBLE QUOTATION MARK
    '\u{201D}', // RIGHT DOUBLE QUOTATION MARK
    '\u{201F}', // DOUBLE HIGH-REVERSED-9 QUOTATION MARK
];

const UTF8_SINGLE_QUOTE_CODEPOINTS: &[char] = &[
    '\u{FF07}', // FULLWIDTH APOSTROPHE
    '\u{2018}', // LEFT SINGLE QUOTATION MARK
    '\u{2019}', // RIGHT SINGLE QUOTATION MARK
    '\u{201B}', // SINGLE HIGH-REVERSED-9 QUOTATION MARK
];

#[derive(Debug, usage_rs::Args)]
#[usage(effect = "write")]
pub struct FixSmartQuotes {
    /// Check for smart quotes without fixing them
    #[usage(long)]
    pub check: bool,

    /// Output a diff of the change (implies `--check`)
    #[usage(short, long)]
    pub diff: bool,

    /// Files to check/fix
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,
}

impl FixSmartQuotes {
    pub async fn run(&self) -> Result<()> {
        let mut found_issues = false;

        for file_path in &self.files {
            if self.diff {
                if let Some(diff) = generate_diff(file_path)? {
                    print!("{}", diff);
                    found_issues = true;
                }
            } else if self.check {
                if has_smart_quotes(file_path)? {
                    println!("{}", file_path.display());
                    found_issues = true;
                }
            } else {
                replace_smart_quotes(file_path)?;
            }
        }

        if (self.check || self.diff) && found_issues {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Reads the file as UTF-8 text. Returns `None` for non-UTF-8 content, which is
/// left untouched because it cannot contain the UTF-8 quote code points we replace.
fn read_utf8(path: &Path) -> Result<Option<String>> {
    Ok(String::from_utf8(fs::read(path)?).ok())
}

fn fix_quotes(s: &str) -> String {
    s.replace(UTF8_DOUBLE_QUOTE_CODEPOINTS, "\"")
        .replace(UTF8_SINGLE_QUOTE_CODEPOINTS, "'")
}

fn has_smart_quotes(path: &PathBuf) -> Result<bool> {
    Ok(read_utf8(path)?.is_some_and(|text| {
        text.contains(UTF8_DOUBLE_QUOTE_CODEPOINTS) || text.contains(UTF8_SINGLE_QUOTE_CODEPOINTS)
    }))
}

fn generate_diff(path: &PathBuf) -> Result<Option<String>> {
    let Some(original) = read_utf8(path)? else {
        return Ok(None);
    };
    let fixed = fix_quotes(&original);
    if fixed == original {
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

fn replace_smart_quotes(path: &PathBuf) -> Result<()> {
    // Non-UTF-8 files are left untouched rather than truncated or rewritten.
    let Some(original) = read_utf8(path)? else {
        return Ok(());
    };
    let fixed = fix_quotes(&original);
    if fixed == original {
        return Ok(());
    }

    // Write through symlinks: replace the target, not the link itself.
    let target = fs::canonicalize(path)?;
    let perms = fs::metadata(&target)?.permissions();
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let mut tmpfile = NamedTempFile::new_in(dir)?;
    tmpfile.write_all(fixed.as_bytes())?;
    tmpfile.as_file().sync_all()?;
    fs::set_permissions(tmpfile.path(), perms)?;
    tmpfile.persist(&target).map_err(|e| e.error)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_replace_smart_quotes() {
        let file = NamedTempFile::new().unwrap();

        let content = r#"
＂FULLWIDTH QUOTATION MARK＂
“LEFT DOUBLE QUOTATION MARK“
”RIGHT DOUBLE QUOTATION MARK”
‟DOUBLE HIGH-REVERSED-9 QUOTATION MARK‟
＇FULLWIDTH APOSTROPHE＇
‘LEFT SINGLE QUOTATION MARK‘
’RIGHT SINGLE QUOTATION MARK’
‛SINGLE HIGH-REVERSED-9 QUOTATION MARK‛
"#;
        fs::write(file.path(), content).unwrap();

        replace_smart_quotes(&file.path().to_path_buf()).unwrap();

        let result_bytes = fs::read(file.path()).unwrap();
        let result = str::from_utf8(&result_bytes).unwrap();
        assert_eq!(
            result,
            r#"
"FULLWIDTH QUOTATION MARK"
"LEFT DOUBLE QUOTATION MARK"
"RIGHT DOUBLE QUOTATION MARK"
"DOUBLE HIGH-REVERSED-9 QUOTATION MARK"
'FULLWIDTH APOSTROPHE'
'LEFT SINGLE QUOTATION MARK'
'RIGHT SINGLE QUOTATION MARK'
'SINGLE HIGH-REVERSED-9 QUOTATION MARK'
"#
        );
    }

    #[test]
    fn test_file_without_smart_quotes_unchanged() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"\"Hello, world!\"").unwrap();

        replace_smart_quotes(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"\"Hello, world!\"");
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"").unwrap();

        replace_smart_quotes(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"");
    }

    #[test]
    fn test_file_only_smart_quotes() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "＂＂").unwrap();

        replace_smart_quotes(&file.path().to_path_buf()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"\"\"");
    }

    #[test]
    #[cfg(unix)]
    fn test_preserve_file_permissions() {
        let file = NamedTempFile::new().unwrap();

        // Change file to be read-only to validate file permissions are correctly preserved.
        let mut before = fs::metadata(file.path()).unwrap().permissions();
        before.set_readonly(true);
        fs::set_permissions(file.path(), before).unwrap();

        replace_smart_quotes(&file.path().to_path_buf()).unwrap();

        let after = fs::metadata(file.path()).unwrap().permissions();
        assert!(after.readonly());
    }

    #[test]
    fn test_has_smart_quotes_true() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "This has \u{201C}smart quotes\u{201D}").unwrap();

        assert!(has_smart_quotes(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    fn test_has_smart_quotes_false() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "This has \"normal quotes\"").unwrap();

        assert!(!has_smart_quotes(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    fn test_has_smart_quotes_empty() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "").unwrap();

        assert!(!has_smart_quotes(&file.path().to_path_buf()).unwrap());
    }

    #[test]
    fn test_non_utf8_file_left_untouched() {
        let file = NamedTempFile::new().unwrap();
        let mut content = b"\xff\xfe line one\n".to_vec();
        content.extend_from_slice("\u{201C}quoted\u{201D}\n".as_bytes());
        content.extend_from_slice(b"line three \x80\n");
        fs::write(file.path(), &content).unwrap();

        let p = file.path().to_path_buf();
        replace_smart_quotes(&p).unwrap();
        assert_eq!(fs::read(file.path()).unwrap(), content);
        assert!(!has_smart_quotes(&p).unwrap());
        assert!(generate_diff(&p).unwrap().is_none());
    }

    #[test]
    #[cfg(unix)]
    fn test_symlink_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.txt");
        let link = dir.path().join("link.txt");
        fs::write(&target, "\u{201C}hi\u{201D}").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        replace_smart_quotes(&link).unwrap();

        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "\"hi\"");
    }

    #[test]
    #[cfg(unix)]
    fn test_preserve_executable_mode() {
        use std::os::unix::fs::PermissionsExt;
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "\u{201C}x\u{201D}").unwrap();
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o755)).unwrap();

        replace_smart_quotes(&file.path().to_path_buf()).unwrap();

        let mode = fs::metadata(file.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
    }

    #[test]
    fn test_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(replace_smart_quotes(&dir.path().join("nope")).is_err());
    }
}
