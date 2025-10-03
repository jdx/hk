use crate::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, clap::Args)]
pub struct CheckCaseConflict {
    /// Files to check for case conflicts
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl CheckCaseConflict {
    pub async fn run(&self) -> Result<()> {
        // Get all files from the repo
        let repo_files = get_repo_files()?;

        // Combine repo files with files being checked
        let mut all_files = repo_files;
        all_files.extend(self.files.iter().cloned());

        let conflicts = find_case_conflicts(&all_files);

        if !conflicts.is_empty() {
            for conflict_group in conflicts {
                println!("Case conflict:");
                for file in conflict_group {
                    println!("  {}", file.display());
                }
            }
            std::process::exit(1);
        }

        Ok(())
    }
}

fn get_repo_files() -> Result<Vec<PathBuf>> {
    // Try to get files from git
    let output = Command::new("git").args(["ls-files"]).output();

    match output {
        Ok(output) if output.status.success() => {
            let files = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(PathBuf::from)
                .collect();
            Ok(files)
        }
        _ => {
            // If not in a git repo or git command fails, just return empty
            Ok(vec![])
        }
    }
}

fn find_case_conflicts(files: &[PathBuf]) -> Vec<Vec<PathBuf>> {
    let mut case_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

    // Group files by their lowercase representation
    for file in files {
        let lowercase = file.to_string_lossy().to_lowercase();
        case_map.entry(lowercase).or_default().push(file.clone());
    }

    // Filter to only groups with conflicts (2+ files)
    case_map
        .into_values()
        .filter(|group| group.len() > 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_no_conflicts() {
        let files = vec![
            PathBuf::from("file1.txt"),
            PathBuf::from("file2.txt"),
            PathBuf::from("dir/file3.txt"),
        ];
        let conflicts = find_case_conflicts(&files);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_simple_conflict() {
        let files = vec![PathBuf::from("README.md"), PathBuf::from("readme.md")];
        let conflicts = find_case_conflicts(&files);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].len(), 2);
    }

    #[test]
    fn test_multiple_conflicts() {
        let files = vec![
            PathBuf::from("File1.txt"),
            PathBuf::from("file1.txt"),
            PathBuf::from("FILE1.TXT"),
            PathBuf::from("Other.md"),
            PathBuf::from("other.md"),
        ];
        let conflicts = find_case_conflicts(&files);
        assert_eq!(conflicts.len(), 2);

        // Check that we have one group of 3 and one group of 2
        let sizes: Vec<usize> = conflicts.iter().map(|g| g.len()).collect();
        assert!(sizes.contains(&3));
        assert!(sizes.contains(&2));
    }

    #[test]
    fn test_path_with_directory() {
        let files = vec![
            PathBuf::from("src/Main.rs"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("src/lib.rs"),
        ];
        let conflicts = find_case_conflicts(&files);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].len(), 2);
    }

    #[test]
    fn test_no_conflict_different_dirs() {
        let files = vec![
            PathBuf::from("dir1/file.txt"),
            PathBuf::from("dir2/file.txt"),
        ];
        let conflicts = find_case_conflicts(&files);
        // These don't conflict because full paths are different
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_different_extensions() {
        let files = vec![PathBuf::from("File.txt"), PathBuf::from("file.TXT")];
        let conflicts = find_case_conflicts(&files);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].len(), 2);
    }
}
