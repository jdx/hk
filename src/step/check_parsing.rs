//! Parsing output from check commands.
//!
//! This module handles parsing the output of `check_list_files` and `check_diff`
//! commands to extract the list of files that need to be fixed.
//!
//! - `check_list_files`: Outputs one file path per line
//! - `check_diff`: Outputs unified diff format, files extracted from `---` and `+++` lines

use indexmap::IndexSet;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use xx::file::display_path;

use super::normalize_diff_paths;
use super::types::Step;

/// Resolve a path from check command stdout.
///
/// If the step ran with `dir` (e.g. in a subproject) and the path is relative
/// and doesn't exist in the current working directory, resolve it against `dir`.
fn resolve_path(raw: &str, dir: Option<&str>) -> PathBuf {
    let p = Path::new(raw.trim());
    let p = p.strip_prefix("./").unwrap_or(p);
    match dir {
        Some(d) if !p.is_absolute() && !p.exists() => Path::new(d).join(p),
        _ => p.to_path_buf(),
    }
}

/// Attempt to canonicalize a path, falling back to the original if it fails.
///
/// This is useful for comparing paths that may have been deleted or renamed,
/// where canonicalization would fail but we still want to match them.
pub(crate) fn try_canonicalize(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(err) => {
            warn!("failed to canonicalize file: {} {err}", display_path(path));
            path.to_path_buf()
        }
    }
}

impl Step {
    /// Parse check_list_files output to extract files needing fixes.
    ///
    /// The command outputs one file path per line. This function:
    /// 1. Parses each line as a file path, resolving relative to `self.dir` if needed
    /// 2. Canonicalizes paths for comparison
    /// 3. Filters to only include files from the original input
    ///
    /// # Arguments
    ///
    /// * `original_files` - The files that were passed to the check command
    /// * `stdout` - The stdout output from check_list_files
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// * Files from original_files that were listed in the output
    /// * Extra files that were listed but not in original_files (warnings)
    pub(crate) fn filter_files_from_check_list(
        &self,
        original_files: &[PathBuf],
        stdout: &str,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let dir = self.dir.as_deref();
        let listed: HashSet<PathBuf> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|p| try_canonicalize(&resolve_path(p, dir)))
            .collect();
        let files: IndexSet<PathBuf> = original_files
            .iter()
            .filter(|f| listed.contains(&try_canonicalize(f)))
            .cloned()
            .collect();
        let canonicalized_files: IndexSet<PathBuf> =
            files.iter().map(|f| try_canonicalize(f)).collect();
        let extras: Vec<PathBuf> = listed
            .into_iter()
            .filter(|f| !canonicalized_files.contains(f))
            .collect();
        (files.into_iter().collect(), extras)
    }

    /// Parse unified diff output to extract files needing fixes.
    ///
    /// Extracts file paths from `---` and `+++` lines in unified diff format.
    /// Handles both standard diff output and git-style diffs with `a/` and `b/` prefixes,
    /// resolving paths relative to `self.dir` if needed.
    ///
    /// Also handles timestamp suffixes (e.g., `--- file.py\t2025-01-01 12:00:00`).
    ///
    /// # Arguments
    ///
    /// * `original_files` - The files that were passed to the check command
    /// * `stdout` - The stdout output containing unified diff
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// * Files from original_files that appear in the diff
    /// * Extra files in the diff but not in original_files (warnings)
    pub(crate) fn filter_files_from_check_diff(
        &self,
        original_files: &[PathBuf],
        stdout: &str,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let stdout = normalize_diff_paths(stdout);

        // Parse unified diff format to extract file names from --- and +++ lines
        let mut listed: HashSet<PathBuf> = HashSet::new();

        // First pass: detect if this diff uses a/ and b/ prefixes (git-style)
        let mut has_a_prefix = false;
        let mut has_b_prefix = false;
        for line in stdout.lines() {
            if line.starts_with("--- a/") {
                has_a_prefix = true;
            } else if line.starts_with("+++ b/") {
                has_b_prefix = true;
            }
            if has_a_prefix && has_b_prefix {
                break;
            }
        }
        let should_strip_prefixes = has_a_prefix && has_b_prefix;

        // Second pass: extract file paths
        for line in stdout.lines() {
            if line.starts_with("--- ") {
                if let Some(path_str) = line.strip_prefix("--- ") {
                    // Strip timestamp if present (tab-separated: "--- file.py	2025-01-01 12:00:00")
                    let path = if let Some((before_tab, _)) = path_str.split_once('\t') {
                        before_tab.trim()
                    } else {
                        path_str.trim()
                    };
                    // Strip standard diff path prefixes (a/ or b/) if detected
                    let path = if should_strip_prefixes {
                        path.strip_prefix("a/")
                            .or_else(|| path.strip_prefix("b/"))
                            .unwrap_or(path)
                    } else {
                        path
                    };
                    listed.insert(try_canonicalize(&PathBuf::from(path)));
                }
            } else if line.starts_with("+++ ")
                && let Some(path_str) = line.strip_prefix("+++ ")
            {
                let path = if let Some((before_tab, _)) = path_str.split_once('\t') {
                    before_tab.trim()
                } else {
                    path_str.trim()
                };
                // Strip standard diff path prefixes (a/ or b/) if detected
                let path = if should_strip_prefixes {
                    path.strip_prefix("a/")
                        .or_else(|| path.strip_prefix("b/"))
                        .unwrap_or(path)
                } else {
                    path
                };
                listed.insert(try_canonicalize(&PathBuf::from(path)));
            }
        }
        let files: IndexSet<PathBuf> = original_files
            .iter()
            .filter(|f| listed.contains(&try_canonicalize(f)))
            .cloned()
            .collect();
        let canonicalized_files: IndexSet<PathBuf> =
            files.iter().map(|f| try_canonicalize(f)).collect();
        let extras: Vec<PathBuf> = listed
            .into_iter()
            .filter(|f| !canonicalized_files.contains(f))
            .collect();
        (files.into_iter().collect(), extras)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_files_from_check_list_subproject() {
        let mut step = Step::default();
        step.dir = Some("frontend".to_string());
        let original_files = vec![
            PathBuf::from("frontend/src/App.tsx"),
            PathBuf::from("frontend/src/index.ts"),
        ];
        let stdout = "src/App.tsx\n";
        let (files, extras) = step.filter_files_from_check_list(&original_files, stdout);
        assert_eq!(files, vec![PathBuf::from("frontend/src/App.tsx")]);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }

    #[test]
    fn test_filter_files_from_check_list_subproject_with_dot_slash() {
        let mut step = Step::default();
        step.dir = Some("frontend".to_string());
        let original_files = vec![
            PathBuf::from("frontend/src/App.tsx"),
            PathBuf::from("frontend/src/index.ts"),
        ];
        let stdout = "./src/App.tsx\n";
        let (files, extras) = step.filter_files_from_check_list(&original_files, stdout);
        assert_eq!(files, vec![PathBuf::from("frontend/src/App.tsx")]);
        assert_eq!(extras, Vec::<PathBuf>::new());
    }
}
