use crate::Result;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

/// Check for merge conflict markers in files
#[derive(Debug, clap::Args)]
pub struct CheckMergeConflict {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl CheckMergeConflict {
    pub async fn run(&self) -> Result<()> {
        let mut found_conflicts = false;

        for file_path in &self.files {
            if has_merge_conflict_markers(file_path)? {
                println!("{}", file_path.display());
                found_conflicts = true;
            }
        }

        if found_conflicts {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Check if a file has merge conflict markers
fn has_merge_conflict_markers(path: &PathBuf) -> Result<bool> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        // Check for conflict markers at the start of the line
        if trimmed.starts_with("<<<<<<<")
            || trimmed.starts_with("=======")
            || trimmed.starts_with(">>>>>>>")
        {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_has_merge_conflict_markers() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "normal line").unwrap();
        writeln!(file, "<<<<<<< HEAD").unwrap();
        writeln!(file, "my changes").unwrap();
        writeln!(file, "=======").unwrap();
        writeln!(file, "their changes").unwrap();
        writeln!(file, ">>>>>>> branch").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(has_merge_conflict_markers(&path).unwrap());
    }

    #[test]
    fn test_no_merge_conflict_markers() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "normal line").unwrap();
        writeln!(file, "another line").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_merge_conflict_markers(&path).unwrap());
    }

    #[test]
    fn test_merge_conflict_markers_with_whitespace() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "  <<<<<<< HEAD  ").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(has_merge_conflict_markers(&path).unwrap());
    }

    #[test]
    fn test_ignores_markers_in_middle() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "this is not <<<<<<< a conflict").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_merge_conflict_markers(&path).unwrap());
    }
}
