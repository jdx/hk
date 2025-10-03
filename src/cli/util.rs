use crate::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Utility commands for file operations
#[derive(Debug, clap::Args)]
pub struct Util {
    #[clap(subcommand)]
    command: UtilCommands,
}

#[derive(Debug, clap::Subcommand)]
enum UtilCommands {
    /// Check for and optionally fix trailing whitespace
    TrailingWhitespace(TrailingWhitespace),
}

/// Check for and optionally fix trailing whitespace in files
#[derive(Debug, clap::Args)]
pub struct TrailingWhitespace {
    /// Fix trailing whitespace by removing it
    #[clap(short, long)]
    fix: bool,

    /// Files to check/fix
    #[clap(required = true)]
    files: Vec<PathBuf>,
}

impl Util {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            UtilCommands::TrailingWhitespace(cmd) => cmd.run().await,
        }
    }
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
                // Fix mode: remove trailing whitespace
                if fix_trailing_whitespace(file_path)? {
                    found_issues = true;
                }
            } else {
                // Check mode: report files with trailing whitespace
                if has_trailing_whitespace(file_path)? {
                    println!("{}", file_path.display());
                    found_issues = true;
                }
            }
        }

        if found_issues {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Check if a file is a text file
fn is_text_file(path: &PathBuf) -> Result<bool> {
    if !path.exists() || !path.is_file() {
        return Ok(false);
    }

    // Try to determine if it's a text file by reading MIME type
    let output = std::process::Command::new("file")
        .arg("-b")
        .arg("--mime-type")
        .arg(path)
        .output()?;

    let mime_type = String::from_utf8_lossy(&output.stdout);
    Ok(mime_type.starts_with("text/"))
}

/// Check if a file has trailing whitespace
fn has_trailing_whitespace(path: &PathBuf) -> Result<bool> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.ends_with(char::is_whitespace) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Fix trailing whitespace in a file, returns true if file was modified
fn fix_trailing_whitespace(path: &PathBuf) -> Result<bool> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = Vec::new();
    let mut modified = false;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim_end();
        if trimmed.len() != line.len() {
            modified = true;
        }
        lines.push(trimmed.to_string());
    }

    if modified {
        let mut file = fs::File::create(path)?;
        for line in lines {
            writeln!(file, "{}", line)?;
        }
    }

    Ok(modified)
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
}
