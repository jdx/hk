//! File filtering and profile handling for steps.
//!
//! This module contains logic for:
//! - Binary file detection with caching
//! - Symlink detection with caching
//! - Profile-based step enabling/disabling
//! - File filtering based on globs, types, and exclusions

use crate::hook::SkipReason;
use crate::par::PathMemo;
use crate::settings::Settings;
use crate::{Result, glob};
use dashmap::DashMap;
use indexmap::IndexSet;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use super::{FileSelector, Pattern, Step};

/// Check if a file is binary by reading the first 8KB and looking for null bytes.
///
/// Results are cached using a lock-free DashMap to avoid repeated filesystem reads
/// and mutex bottlenecks in concurrent scenarios.
///
/// # Arguments
///
/// * `path` - Path to the file to check
///
/// # Returns
///
/// * `Some(true)` - File is binary
/// * `Some(false)` - File is text
/// * `None` - Could not read file (deleted, permissions, etc.)
pub fn is_binary_file(path: &PathBuf) -> Option<bool> {
    // Memoize results (only cache successful reads, not errors). Steps that
    // filter the same files at once share one read of each file.
    static CACHE: LazyLock<PathMemo<bool>> = LazyLock::new(PathMemo::new);

    CACHE.get_or_try_init(path, || {
        let mut file = std::fs::File::open(path).ok()?;
        let mut buffer = [0u8; 8192];
        let bytes_read = file.read(&mut buffer).ok()?;

        // Check for null bytes in the content
        Some(buffer[..bytes_read].contains(&0))
    })
}

/// Check if a file is a symbolic link.
///
/// Results are cached using a lock-free DashMap to avoid repeated filesystem reads
/// and mutex bottlenecks in concurrent scenarios.
///
/// # Arguments
///
/// * `path` - Path to check
///
/// # Returns
///
/// * `Some(true)` - Path is a symlink
/// * `Some(false)` - Path is not a symlink
/// * `None` - Could not read metadata (deleted, permissions, etc.)
pub fn is_symlink_file(path: &PathBuf) -> Option<bool> {
    // Check cache first (lock-free read)
    if let Some(result) = SYMLINK_CACHE.get(path) {
        return Some(*result);
    }

    let metadata = std::fs::symlink_metadata(path).ok()?;
    Some(cache_symlink_check(path, &metadata))
}

/// Memoized [`is_symlink_file`] results (only successful reads, not errors).
/// DashMap provides lock-free concurrent access, avoiding Mutex bottlenecks.
static SYMLINK_CACHE: LazyLock<DashMap<PathBuf, bool>> = LazyLock::new(DashMap::new);

/// Record the result of [`is_symlink_file`] from `lstat` metadata read
/// elsewhere, so file type detection and the symlink filter share one
/// `lstat` per file.
pub(crate) fn cache_symlink_check(path: &Path, metadata: &std::fs::Metadata) -> bool {
    let is_symlink = metadata.file_type().is_symlink();
    SYMLINK_CACHE.insert(path.to_path_buf(), is_symlink);
    is_symlink
}

impl Step {
    fn filter_match_any_selector(
        &self,
        files: &[PathBuf],
        selector: &FileSelector,
    ) -> Result<Vec<PathBuf>> {
        self.filter_selector(files, selector.glob.as_ref(), selector.types.as_deref())
    }

    fn filter_selector(
        &self,
        files: &[PathBuf],
        pattern: Option<&Pattern>,
        types: Option<&[String]>,
    ) -> Result<Vec<PathBuf>> {
        let mut files = files.to_vec();
        if let Some(pattern) = pattern {
            files = glob::get_pattern_matches(pattern, &files, self.dir_prefix())?;
        }
        if let Some(types) = types {
            crate::par::retain(&mut files, |f| crate::file_type::matches_types(f, types));
        }
        Ok(files)
    }

    /// Get the profiles that enable this step.
    ///
    /// Returns profiles from the `profiles` field that don't start with `!`.
    pub fn enabled_profiles(&self) -> Option<IndexSet<String>> {
        self.profiles.as_ref().map(|profiles| {
            profiles
                .iter()
                .filter(|s| !s.starts_with('!'))
                .map(|s| s.to_string())
                .collect()
        })
    }

    /// Get the profiles that disable this step.
    ///
    /// Returns profiles from the `profiles` field that start with `!` (with the `!` stripped).
    pub fn disabled_profiles(&self) -> Option<IndexSet<String>> {
        self.profiles.as_ref().map(|profiles| {
            profiles
                .iter()
                .filter(|s| s.starts_with('!'))
                .map(|s| s.strip_prefix('!').unwrap().to_string())
                .collect()
        })
    }

    /// Determine if this step should be skipped based on profile settings.
    ///
    /// Checks if:
    /// - Required profiles are not enabled
    /// - Explicitly disabled profiles are enabled
    ///
    /// # Returns
    ///
    /// `Some(SkipReason)` if the step should be skipped, `None` if it should run
    pub fn profile_skip_reason(&self) -> Option<SkipReason> {
        let settings = Settings::get();
        if let Some(enabled) = self.enabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let missing_profiles = enabled.difference(&enabled_profiles).collect::<Vec<_>>();
            if !missing_profiles.is_empty() {
                let profiles = missing_profiles
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                return Some(SkipReason::ProfileNotEnabled(profiles));
            }
            let disabled_profiles_set = settings.disabled_profiles();
            let disabled_profiles = disabled_profiles_set.intersection(&enabled).collect_vec();
            if !disabled_profiles.is_empty() {
                return Some(SkipReason::ProfileExplicitlyDisabled);
            }
        }
        if let Some(disabled) = self.disabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let disabled_profiles = disabled.intersection(&enabled_profiles).collect::<Vec<_>>();
            if !disabled_profiles.is_empty() {
                return Some(SkipReason::ProfileExplicitlyDisabled);
            }
        }
        None
    }

    /// Filter a list of files based on the step's configuration.
    ///
    /// Applies the following filters in order:
    /// 1. Directory filter (`dir`) - only files under this directory
    ///    (its literal prefix when templated, see [`Step::dir_prefix`])
    /// 2. Positive selectors (`glob` + `types`, or `match_any` clauses)
    /// 3. Exclusion pattern (`exclude`) - must not match
    /// 4. Binary filter (`allow_binary`) - skip binary files unless allowed
    /// 5. Symlink filter (`allow_symlinks`) - skip symlinks unless allowed
    ///
    /// # Arguments
    ///
    /// * `files` - The list of files to filter
    ///
    /// # Returns
    ///
    /// The filtered list of files that match all criteria
    pub fn filter_files(&self, files: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut files = files.to_vec();
        if let Some(dir) = self.dir_prefix() {
            files.retain(|f| f.starts_with(dir));
            if files.is_empty() {
                debug!("{self}: no files in {dir}");
            }
            // Don't strip the dir prefix here - it causes issues when steps have different working directories
            // The path stripping should only happen in the command execution context via tera templates
        }
        files = if let Some(selectors) = &self.match_any {
            let mut matched = HashSet::new();
            for selector in selectors {
                matched.extend(self.filter_match_any_selector(&files, selector)?);
            }
            files
                .into_iter()
                .filter(|file| matched.contains(file))
                .collect()
        } else {
            self.filter_selector(&files, self.glob.as_ref(), self.types.as_deref())?
        };
        if let Some(pattern) = self.exclude.as_ref().filter(|pattern| !pattern.is_empty()) {
            // Use get_pattern_matches consistently for excludes too
            let excluded: HashSet<_> =
                glob::get_pattern_matches(pattern, &files, self.dir_prefix())?
                    .into_iter()
                    .collect();
            files.retain(|f| !excluded.contains(f));
        }

        // Filter out binary files unless allow_binary is true
        if !self.allow_binary {
            crate::par::retain(&mut files, |f| {
                // Keep file if we can't determine if it's binary (might be deleted/renamed)
                // or if it's definitely not binary
                is_binary_file(f).map(|is_bin| !is_bin).unwrap_or(true)
            });
        }

        // Filter out symbolic links unless allow_symlinks is true
        if !self.allow_symlinks {
            crate::par::retain(&mut files, |f| {
                // Keep file if we can't determine if it's a symlink (might be deleted/renamed)
                // or if it's definitely not a symlink
                is_symlink_file(f)
                    .map(|is_symlink| !is_symlink)
                    .unwrap_or(true)
            });
        }

        Ok(files)
    }

    /// Find workspace roots for a list of files.
    ///
    /// For monorepo-style projects, this identifies which workspace each file belongs to
    /// by searching up the directory tree for the workspace indicator file (e.g., `Cargo.toml`).
    ///
    /// # Arguments
    ///
    /// * `files` - List of files to find workspaces for
    ///
    /// # Returns
    ///
    /// * `Ok(Some(workspaces))` - Set of workspace indicator file paths found
    /// * `Ok(None)` - No workspace_indicator configured for this step
    ///
    /// # Example
    ///
    /// For files like:
    /// - `src/crate-1/src/lib.rs`
    /// - `src/crate-2/src/lib.rs`
    ///
    /// With `workspace_indicator = "Cargo.toml"`, returns:
    /// - `src/crate-1/Cargo.toml`
    /// - `src/crate-2/Cargo.toml`
    pub fn workspaces_for_files(&self, files: &[PathBuf]) -> Result<Option<IndexSet<PathBuf>>> {
        let Some(workspace_indicator) = &self.workspace_indicator else {
            return Ok(None);
        };
        let mut finder = WorkspaceFinder::new(workspace_indicator);
        let owners = finder.owners(files);
        Ok(Some(workspaces_in_listing_order(&owners)))
    }

    /// Split `files` into one group per workspace, each file going to the
    /// nearest workspace above it. Files with no workspace are left out.
    ///
    /// Groups come longest workspace path first, so a nested workspace comes
    /// before the one that contains it. Each group keeps the order of `files`.
    /// Returns no groups when no workspace_indicator is configured.
    pub(super) fn workspace_groups(&self, files: Vec<PathBuf>) -> Vec<(PathBuf, Vec<PathBuf>)> {
        let Some(workspace_indicator) = &self.workspace_indicator else {
            return vec![];
        };
        let mut finder = WorkspaceFinder::new(workspace_indicator);
        let owners = finder.owners(&files);
        // Longest first. The sort is stable, so equally long paths keep the
        // order in which they were found.
        let workspaces: Vec<PathBuf> = workspaces_in_listing_order(&owners)
            .into_iter()
            .sorted_by(|a, b| b.as_os_str().len().cmp(&a.as_os_str().len()))
            .collect();
        let position: HashMap<&PathBuf, usize> = workspaces
            .iter()
            .enumerate()
            .map(|(i, workspace)| (workspace, i))
            .collect();
        let mut groups: Vec<Vec<PathBuf>> = vec![vec![]; workspaces.len()];
        for (file, owner) in files.into_iter().zip(&owners) {
            if let Some(owner) = owner {
                groups[position[owner]].push(file);
            }
        }
        workspaces.into_iter().zip(groups).collect()
    }
}

/// The workspaces in `owners`, in the order [`Step::workspaces_for_files`]
/// lists them: those of the last files first.
fn workspaces_in_listing_order(owners: &[Option<PathBuf>]) -> IndexSet<PathBuf> {
    owners.iter().rev().flatten().cloned().collect()
}

/// Finds the nearest directory at or above a file that holds a workspace
/// indicator, remembering what it learned about each directory.
///
/// Files in a monorepo share a few directories, so each directory is looked at
/// once instead of once per file, and a walk up stops at the first directory
/// already known.
struct WorkspaceFinder<'a> {
    indicator: &'a str,
    /// The indicator found at or above each directory looked at so far
    found: HashMap<PathBuf, Option<PathBuf>>,
}

impl<'a> WorkspaceFinder<'a> {
    fn new(indicator: &'a str) -> Self {
        Self {
            indicator,
            found: HashMap::new(),
        }
    }

    /// The indicator nearest above each file's directory, as
    /// [`xx::file::find_up`] finds it. A file without a directory has none.
    fn owners(&mut self, files: &[PathBuf]) -> Vec<Option<PathBuf>> {
        files
            .iter()
            .map(|file| file.parent().and_then(|dir| self.find(dir)))
            .collect()
    }

    fn find(&mut self, dir: &Path) -> Option<PathBuf> {
        if let Some(found) = self.found.get(dir) {
            return found.clone();
        }
        // The directories passed on the way up, which share the answer
        let mut passed: Vec<&Path> = vec![];
        let mut current = Some(dir);
        let found = loop {
            let Some(dir) = current else {
                break None;
            };
            if let Some(found) = self.found.get(dir) {
                break found.clone();
            }
            let candidate = dir.join(self.indicator);
            if candidate.exists() {
                self.found
                    .insert(dir.to_path_buf(), Some(candidate.clone()));
                break Some(candidate);
            }
            passed.push(dir);
            current = dir.parent();
        };
        for dir in passed {
            self.found.insert(dir.to_path_buf(), found.clone());
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree of `Cargo.toml` workspaces: `a` holds `a/b`, `d` and `g/h` stand
    /// alone, and `nowork` and `root.rs` are in none.
    fn workspace_tree() -> (tempfile::TempDir, Vec<PathBuf>) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for workspace in ["a", "a/b", "d", "g/h"] {
            std::fs::create_dir_all(root.join(workspace)).unwrap();
            std::fs::write(root.join(workspace).join("Cargo.toml"), "").unwrap();
        }
        let files = [
            "a/1.rs",
            "a/b/c/deep/4.rs",
            "nowork/z/10.rs",
            "a/b/2.rs",
            "d/e/f/6.rs",
            "g/9.rs",
            "g/h/8.rs",
            "a/x/5.rs",
            "root.rs",
        ]
        .map(|f| root.join(f))
        .to_vec();
        (dir, files)
    }

    fn workspace_step() -> Step {
        Step {
            workspace_indicator: Some("Cargo.toml".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn workspaces_for_files_lists_those_of_the_last_files_first() {
        let (dir, files) = workspace_tree();
        let root = dir.path();
        let workspaces = workspace_step().workspaces_for_files(&files).unwrap();
        assert_eq!(
            workspaces.unwrap().into_iter().collect_vec(),
            ["a", "g/h", "d", "a/b"].map(|w| root.join(w).join("Cargo.toml"))
        );
        assert_eq!(Step::default().workspaces_for_files(&files).unwrap(), None);
    }

    #[test]
    fn workspace_groups_give_each_file_its_nearest_workspace() {
        let (dir, files) = workspace_tree();
        let root = dir.path();
        let group = |workspace: &str, files: &[&str]| {
            (
                root.join(workspace).join("Cargo.toml"),
                files.iter().map(|f| root.join(f)).collect_vec(),
            )
        };
        // Longest workspace path first, equally long ones in the order they
        // were found; files without a workspace are left out
        assert_eq!(
            workspace_step().workspace_groups(files),
            vec![
                group("g/h", &["g/h/8.rs"]),
                group("a/b", &["a/b/c/deep/4.rs", "a/b/2.rs"]),
                group("a", &["a/1.rs", "a/x/5.rs"]),
                group("d", &["d/e/f/6.rs"]),
            ]
        );
    }

    #[test]
    fn match_any_preserves_input_order() {
        let step = Step {
            match_any: Some(vec![
                FileSelector {
                    glob: Some(Pattern::Globs(vec!["**/*.bats".to_string()])),
                    types: None,
                },
                FileSelector {
                    glob: Some(Pattern::Globs(vec!["**/*.sh".to_string()])),
                    types: None,
                },
            ]),
            ..Default::default()
        };
        let files = vec![PathBuf::from("first.sh"), PathBuf::from("second.bats")];

        assert_eq!(step.filter_files(&files).unwrap(), files);
    }
}
