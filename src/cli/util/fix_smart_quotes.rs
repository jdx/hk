use crate::Result;
use std::fs;
use std::io::{BufWriter, Read, Write};
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

/// Bytes read per step. Memory use is bounded by this, whatever the file's line lengths.
const CHUNK_SIZE: usize = 64 * 1024;

fn is_smart_quote(c: char) -> bool {
    UTF8_DOUBLE_QUOTE_CODEPOINTS.contains(&c) || UTF8_SINGLE_QUOTE_CODEPOINTS.contains(&c)
}

fn fix_quotes(s: &str) -> String {
    s.replace(UTF8_DOUBLE_QUOTE_CODEPOINTS, "\"")
        .replace(UTF8_SINGLE_QUOTE_CODEPOINTS, "'")
}

/// Calls `f` with successive pieces of the file, each valid UTF-8 that starts and
/// ends on a character boundary, so a smart quote is never split between two
/// pieces. Only `chunk_size` bytes plus an incomplete trailing sequence (at most 3
/// bytes) are held at once. Returns `false` if the file is not valid UTF-8
/// (including a truncated sequence at EOF); `f` may already have seen earlier pieces.
fn for_each_chunk(
    path: &Path,
    chunk_size: usize,
    mut f: impl FnMut(&str) -> Result<()>,
) -> Result<bool> {
    let mut file = fs::File::open(path)?;
    let mut chunk = vec![0u8; chunk_size.max(1)];
    let mut buf: Vec<u8> = Vec::with_capacity(chunk.len() + 4);
    loop {
        let n = match file.read(&mut chunk) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        };
        let eof = n == 0;
        buf.extend_from_slice(&chunk[..n]);
        match std::str::from_utf8(&buf) {
            Ok(s) => {
                if !s.is_empty() {
                    f(s)?;
                }
                buf.clear();
            }
            Err(e) => {
                // `error_len() == None` means the input just ended mid-character,
                // which is only acceptable if more bytes follow.
                if e.error_len().is_some() || eof {
                    return Ok(false);
                }
                let valid = e.valid_up_to();
                if valid > 0 {
                    f(std::str::from_utf8(&buf[..valid]).expect("validated prefix"))?;
                }
                buf.drain(..valid);
            }
        }
        if eof {
            return Ok(true);
        }
    }
}

/// Streams the whole file once. The scan must reach the end, even after finding a
/// quote, so that check and fix agree: a file that is not entirely UTF-8 is never
/// reported and never rewritten.
fn has_smart_quotes_in(path: &Path, chunk_size: usize) -> Result<bool> {
    let mut found = false;
    let utf8 = for_each_chunk(path, chunk_size, |s| {
        found = found || s.contains(is_smart_quote);
        Ok(())
    })?;
    Ok(utf8 && found)
}

fn has_smart_quotes(path: &Path) -> Result<bool> {
    has_smart_quotes_in(path, CHUNK_SIZE)
}

fn generate_diff(path: &Path) -> Result<Option<String>> {
    generate_diff_with(path, CHUNK_SIZE, |p| Ok(fs::read(p)?))
}

/// Pass 1 is a read-only bounded-memory scan that decides "valid UTF-8 with smart
/// quotes"; clean and non-UTF-8 files return here without being loaded. Only a file
/// that needs changes is loaded (pass 2, `load`) because a diff needs both texts.
fn generate_diff_with(
    path: &Path,
    chunk_size: usize,
    load: impl FnOnce(&Path) -> Result<Vec<u8>>,
) -> Result<Option<String>> {
    if !has_smart_quotes_in(path, chunk_size)? {
        return Ok(None);
    }
    let Ok(original) = String::from_utf8(load(path)?) else {
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

fn replace_smart_quotes(path: &Path) -> Result<()> {
    replace_smart_quotes_in(path, CHUNK_SIZE)
}

/// Pass 1 is a read-only scan: clean and non-UTF-8 files return with no writes, so
/// they need no writable directory and no scratch space. Only a file that needs
/// changes gets pass 2, which streams the fixed text into a temp file next to the
/// target and atomically replaces the target once the whole file read as UTF-8
/// (if it stopped being UTF-8 in between, the temp file is dropped and the
/// original left alone). Files that change are read twice by design.
fn replace_smart_quotes_in(path: &Path, chunk_size: usize) -> Result<()> {
    if !has_smart_quotes_in(path, chunk_size)? {
        return Ok(());
    }

    // Write through symlinks: replace the target, not the link itself.
    let target = fs::canonicalize(path)?;
    rewrite_target(&target, chunk_size)
}

/// Pass 2. The temp file replaces `target` only if the file is still entirely
/// UTF-8 and this pass actually replaced a quote. If another process cleaned the
/// file (or retargeted a symlink) since pass 1, the temp file is dropped and the
/// target keeps its inode, metadata and hard links.
fn rewrite_target(target: &Path, chunk_size: usize) -> Result<()> {
    let perms = fs::metadata(target)?.permissions();
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let mut tmpfile = NamedTempFile::new_in(dir)?;
    let mut out = BufWriter::new(tmpfile.as_file_mut());
    let mut changed = false;
    let utf8 = for_each_chunk(target, chunk_size, |s| {
        let fixed = fix_quotes(s);
        changed = changed || fixed != s;
        out.write_all(fixed.as_bytes())?;
        Ok(())
    })?;
    out.flush()?;
    drop(out);
    if !utf8 || !changed {
        return Ok(());
    }
    tmpfile.as_file().sync_all()?;
    fs::set_permissions(tmpfile.path(), perms)?;
    tmpfile.persist(target).map_err(|e| e.error)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A file in its own directory with no open handle. `NamedTempFile` keeps one
    /// open, which on Windows blocks replacing the file.
    struct NamedTempFile {
        _dir: tempfile::TempDir,
        path: PathBuf,
    }

    impl NamedTempFile {
        fn new() -> std::io::Result<Self> {
            let dir = tempfile::tempdir()?;
            let path = dir.path().join("file.txt");
            fs::write(&path, b"")?;
            Ok(Self { _dir: dir, path })
        }
        fn path(&self) -> &Path {
            &self.path
        }
    }

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

        replace_smart_quotes(file.path()).unwrap();

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

        replace_smart_quotes(file.path()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"\"Hello, world!\"");
    }

    #[test]
    fn test_empty_file() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), b"").unwrap();

        replace_smart_quotes(file.path()).unwrap();

        let result = fs::read(file.path()).unwrap();
        assert_eq!(result, b"");
    }

    #[test]
    fn test_file_only_smart_quotes() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "＂＂").unwrap();

        replace_smart_quotes(file.path()).unwrap();

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

        replace_smart_quotes(file.path()).unwrap();

        let after = fs::metadata(file.path()).unwrap().permissions();
        assert!(after.readonly());
    }

    #[test]
    fn test_has_smart_quotes_true() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "This has \u{201C}smart quotes\u{201D}").unwrap();

        assert!(has_smart_quotes(file.path()).unwrap());
    }

    #[test]
    fn test_has_smart_quotes_false() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "This has \"normal quotes\"").unwrap();

        assert!(!has_smart_quotes(file.path()).unwrap());
    }

    #[test]
    fn test_has_smart_quotes_empty() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "").unwrap();

        assert!(!has_smart_quotes(file.path()).unwrap());
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

        replace_smart_quotes(file.path()).unwrap();

        let mode = fs::metadata(file.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
    }

    #[test]
    fn test_non_utf8_after_quotes_left_untouched_and_unreported() {
        let file = NamedTempFile::new().unwrap();
        let mut content = "\u{201C}ok\u{201D}\n".repeat(10_000).into_bytes();
        content.extend_from_slice(b"bad \xff\n");
        fs::write(file.path(), &content).unwrap();

        replace_smart_quotes(file.path()).unwrap();
        assert_eq!(fs::read(file.path()).unwrap(), content);
        assert!(!has_smart_quotes(file.path()).unwrap());
    }

    #[test]
    fn test_multiline_file_fixed() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "a\n\u{2018}b\u{2019}\nc").unwrap();
        replace_smart_quotes(file.path()).unwrap();
        assert_eq!(fs::read(file.path()).unwrap(), b"a\n'b'\nc");
    }

    #[test]
    fn test_quotes_split_across_chunk_boundaries() {
        let input = "a\u{201C}b\u{2019}\u{FF02}c\u{00E9}\u{1F600}\u{2018}d\u{FF07}";
        let expected = "a\"b'\"c\u{00E9}\u{1F600}'d'";
        for chunk in 1..=9 {
            let file = NamedTempFile::new().unwrap();
            fs::write(file.path(), input).unwrap();
            assert!(has_smart_quotes_in(file.path(), chunk).unwrap(), "{chunk}");
            replace_smart_quotes_in(file.path(), chunk).unwrap();
            assert_eq!(
                fs::read_to_string(file.path()).unwrap(),
                expected,
                "{chunk}"
            );
        }
    }

    #[test]
    fn test_long_single_line_without_newlines() {
        let file = NamedTempFile::new().unwrap();
        let mut content = "x".repeat(CHUNK_SIZE * 5 + 1);
        content.push('\u{201C}');
        content.push_str(&"y".repeat(CHUNK_SIZE + 2));
        fs::write(file.path(), &content).unwrap();

        assert!(has_smart_quotes(file.path()).unwrap());
        replace_smart_quotes(file.path()).unwrap();
        assert_eq!(
            fs::read_to_string(file.path()).unwrap(),
            content.replace('\u{201C}', "\"")
        );
    }

    #[test]
    fn test_truncated_or_invalid_sequences_untouched() {
        // Truncated at EOF, and invalid mid-file, each after a quote.
        for tail in [&b"\xe2\x80"[..], &b"\xff more"[..]] {
            for chunk in [1, 3, CHUNK_SIZE] {
                let file = NamedTempFile::new().unwrap();
                let mut content = "\u{201C}q\u{201D}".as_bytes().to_vec();
                content.extend_from_slice(tail);
                fs::write(file.path(), &content).unwrap();
                replace_smart_quotes_in(file.path(), chunk).unwrap();
                assert_eq!(fs::read(file.path()).unwrap(), content);
                assert!(!has_smart_quotes_in(file.path(), chunk).unwrap());
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn test_clean_file_in_read_only_dir_succeeds() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clean.txt");
        fs::write(&path, "plain \"text\"").unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o555)).unwrap();

        let result = replace_smart_quotes(&path);

        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        result.unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"plain \"text\"");
    }

    #[test]
    fn test_diff_loads_only_files_with_smart_quotes() {
        let never = |_: &Path| -> Result<Vec<u8>> { panic!("loaded a file that needs no diff") };

        let clean = NamedTempFile::new().unwrap();
        fs::write(clean.path(), "x".repeat(CHUNK_SIZE * 3)).unwrap();
        assert!(
            generate_diff_with(clean.path(), CHUNK_SIZE, never)
                .unwrap()
                .is_none()
        );

        let binary = NamedTempFile::new().unwrap();
        fs::write(binary.path(), b"\xff\xfe\x00 \xe2\x80\x9c").unwrap();
        assert!(
            generate_diff_with(binary.path(), CHUNK_SIZE, never)
                .unwrap()
                .is_none()
        );

        let quoted = NamedTempFile::new().unwrap();
        fs::write(quoted.path(), "\u{201C}q\u{201D}\n").unwrap();
        let mut loaded = false;
        let diff = generate_diff_with(quoted.path(), CHUNK_SIZE, |p| {
            loaded = true;
            Ok(fs::read(p)?)
        })
        .unwrap();
        assert!(loaded && diff.unwrap().contains("+\"q\""));
    }

    #[test]
    #[cfg(unix)]
    fn test_pass_two_on_a_file_cleaned_since_the_scan_keeps_the_inode() {
        use std::os::unix::fs::MetadataExt;
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "already clean").unwrap();
        let before = fs::metadata(file.path()).unwrap();

        rewrite_target(file.path(), CHUNK_SIZE).unwrap();

        let after = fs::metadata(file.path()).unwrap();
        assert_eq!(
            (before.ino(), before.mtime_nsec()),
            (after.ino(), after.mtime_nsec())
        );
        assert_eq!(fs::read(file.path()).unwrap(), b"already clean");
    }

    #[test]
    fn test_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(replace_smart_quotes(&dir.path().join("nope")).is_err());
    }
}
