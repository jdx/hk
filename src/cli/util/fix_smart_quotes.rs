use crate::Result;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

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

#[derive(Debug, clap::Args)]
pub struct FixSmartQuotes {
    /// Files to replace smart quotes in
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl FixSmartQuotes {
    pub async fn run(&self) -> Result<()> {
        for file_path in &self.files {
            replace_smart_quotes(file_path)?;
        }

        Ok(())
    }
}

fn replace_smart_quotes(path: &PathBuf) -> Result<()> {
    let file = File::open(path)?;
    let mut tmpfile = NamedTempFile::new()?;
    let mut reader = BufReader::new(file);
    let mut buf = String::new();

    while let Ok(read) = reader.read_line(&mut buf) {
        if read == 0 {
            break;
        }
        tmpfile.write_all(
            buf.replace(UTF8_DOUBLE_QUOTE_CODEPOINTS, "\"")
                .replace(UTF8_SINGLE_QUOTE_CODEPOINTS, "'")
                .as_bytes(),
        )?;
        buf.clear();
    }

    fs::rename(tmpfile.path(), path)?;

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
        fs::write(file.path(), &content).unwrap();

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
}
