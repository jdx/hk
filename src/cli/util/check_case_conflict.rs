use crate::Result;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct CheckCaseConflict {
    /// Files to check for case conflicts
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,
}

impl CheckCaseConflict {
    pub async fn run(&self) -> Result<()> {
        // Get all files from the repo
        let repo_files = get_repo_files()?;

        // Combine repo files with files being checked, deduplicating to avoid
        // false positives when the same file appears in both lists (e.g., a staged
        // file that's already tracked by git)
        let all_files: Vec<PathBuf> = repo_files
            .into_iter()
            .chain(self.files.iter().cloned())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        let conflicts = find_case_conflicts(&all_files);

        if !conflicts.is_empty() {
            for conflict_group in conflicts {
                println!("Case conflict:");
                for file in conflict_group {
                    println!("  {}", file.display());
                }
            }
            return Err(eyre::eyre!("Case conflicts found in file paths"));
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
    // Every file plus every ancestor directory, so `Foo/a` and `foo/b` conflict
    // on their directory even though the file paths differ.
    let mut paths: BTreeSet<PathBuf> = BTreeSet::new();
    let mut dirs: HashSet<PathBuf> = HashSet::new();
    for file in files {
        paths.insert(file.clone());
        for ancestor in file.ancestors().skip(1) {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            paths.insert(ancestor.to_path_buf());
            dirs.insert(ancestor.to_path_buf());
        }
    }

    let mut case_map: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for path in paths {
        let lowercase = path.to_string_lossy().to_lowercase();
        case_map.entry(lowercase).or_default().push(path);
    }

    case_map
        .into_values()
        .filter(|group| group.len() > 1)
        // A file group whose members differ only through a conflicting parent
        // directory is already reported by that directory's group.
        .filter(|group| {
            group.iter().any(|p| dirs.contains(p))
                || group
                    .iter()
                    .map(|p| p.parent())
                    .collect::<HashSet<_>>()
                    .len()
                    == 1
        })
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

    #[test]
    fn test_directory_level_conflict() {
        let files = vec![PathBuf::from("Foo/a.txt"), PathBuf::from("foo/b.txt")];
        let conflicts = find_case_conflicts(&files);
        assert_eq!(
            conflicts,
            vec![vec![PathBuf::from("Foo"), PathBuf::from("foo")]]
        );
    }

    #[test]
    fn test_nested_directory_conflict_and_same_file_name() {
        let files = vec![PathBuf::from("a/Foo/x.txt"), PathBuf::from("a/foo/x.txt")];
        let conflicts = find_case_conflicts(&files);
        assert_eq!(
            conflicts,
            vec![vec![PathBuf::from("a/Foo"), PathBuf::from("a/foo")]]
        );
    }
}
