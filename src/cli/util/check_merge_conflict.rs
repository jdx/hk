use crate::Result;
use std::fs;
use std::path::PathBuf;

/// Check for merge conflict markers in files
#[derive(Debug, clap::Args)]
pub struct CheckMergeConflict {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,

    /// Run the check even when not in a merge
    #[clap(long)]
    pub assume_in_merge: bool,
}

impl CheckMergeConflict {
    pub async fn run(&self) -> Result<()> {
        // Only check for merge conflicts if we're actually in a merge or assume_in_merge is set
        // This matches pre-commit behavior
        if !self.assume_in_merge && !is_in_merge() {
            return Ok(());
        }

        let mut found_conflicts = false;

        for file_path in &self.files {
            if has_merge_conflict_markers(file_path)? {
                println!("{}", file_path.display());
                found_conflicts = true;
            }
        }

        if found_conflicts {
            return Err(eyre::eyre!("Merge conflict markers found in files"));
        }

        Ok(())
    }
}

/// Check if we're currently in a merge or rebase
fn is_in_merge() -> bool {
    let Ok(output) = xx::process::cmd("git", ["rev-parse", "--git-dir"]).read() else {
        return false;
    };
    let git_dir = output.trim();

    let merge_msg = std::path::Path::new(git_dir).join("MERGE_MSG");
    let merge_head = std::path::Path::new(git_dir).join("MERGE_HEAD");
    let rebase_apply = std::path::Path::new(git_dir).join("rebase-apply");
    let rebase_merge = std::path::Path::new(git_dir).join("rebase-merge");

    // In a merge: MERGE_MSG and MERGE_HEAD both exist
    // In a rebase: rebase-apply or rebase-merge directories exist
    (merge_msg.exists() && merge_head.exists()) || rebase_apply.exists() || rebase_merge.exists()
}

/// Check if a file has merge conflict markers
fn has_merge_conflict_markers(path: &PathBuf) -> Result<bool> {
    use std::io::Read;

    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Check for conflict markers at the start of lines
    // Patterns from pre-commit: '<<<<<<< ', '======= ', '=======\n', '=======\r\n', '>>>>>>> '
    for line in buffer.split(|&b| b == b'\n') {
        if line.starts_with(b"<<<<<<< ")
            || line.starts_with(b">>>>>>> ")
            || line == b"======="
            || line == b"=======\r"
            || line.starts_with(b"======= ")
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
    fn test_ignores_indented_markers() {
        let mut file = NamedTempFile::new().unwrap();
        // Indented markers should not be detected (not actual conflicts)
        writeln!(file, "  <<<<<<< HEAD  ").unwrap();
        writeln!(file, "    =======").unwrap();
        writeln!(
            file,
            "            ================================================================="
        )
        .unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_merge_conflict_markers(&path).unwrap());
    }

    #[test]
    fn test_ignores_markers_in_middle() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "this is not <<<<<<< a conflict").unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        assert!(!has_merge_conflict_markers(&path).unwrap());
    }

    #[test]
    fn test_handles_binary_files() {
        let mut file = NamedTempFile::new().unwrap();
        // Write some binary data that's not valid UTF-8
        file.write_all(&[0xFF, 0xFE, 0xFD, 0xFC]).unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        // Should not error, just return false
        assert!(!has_merge_conflict_markers(&path).unwrap());
    }
}
