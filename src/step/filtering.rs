//! File filtering and profile handling for steps.
//!
//! This module contains logic for:
//! - Binary file detection with caching
//! - Symlink detection with caching
//! - Profile-based step enabling/disabling
//! - File filtering based on globs, types, and exclusions

use crate::hook::SkipReason;
use crate::settings::Settings;
use crate::{Result, glob};
use dashmap::DashMap;
use indexmap::IndexSet;
use itertools::Itertools;
use std::collections::HashSet;
use std::io::Read;
use std::path::PathBuf;
use std::sync::LazyLock;

use super::types::Step;

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
    // Memoize results (only cache successful reads, not errors)
    // DashMap provides lock-free concurrent access, avoiding Mutex bottlenecks
    static CACHE: LazyLock<DashMap<PathBuf, bool>> = LazyLock::new(DashMap::new);

    // Check cache first (lock-free read)
    if let Some(result) = CACHE.get(path) {
        return Some(*result);
    }

    let mut file = std::fs::File::open(path).ok()?;
    let mut buffer = [0u8; 8192];
    let bytes_read = file.read(&mut buffer).ok()?;

    // Check for null bytes in the content
    let is_binary = buffer[..bytes_read].contains(&0);

    // Cache the result
    CACHE.insert(path.clone(), is_binary);

    Some(is_binary)
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
    // Memoize results (only cache successful reads, not errors)
    // DashMap provides lock-free concurrent access, avoiding Mutex bottlenecks
    static CACHE: LazyLock<DashMap<PathBuf, bool>> = LazyLock::new(DashMap::new);

    // Check cache first (lock-free read)
    if let Some(result) = CACHE.get(path) {
        return Some(*result);
    }

    let metadata = std::fs::symlink_metadata(path).ok()?;
    let is_symlink = metadata.file_type().is_symlink();

    // Cache the result
    CACHE.insert(path.clone(), is_symlink);

    Some(is_symlink)
}

impl Step {
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
    /// 2. Glob/regex pattern (`glob`) - must match pattern
    /// 3. Exclusion pattern (`exclude`) - must not match
    /// 4. Binary filter (`allow_binary`) - skip binary files unless allowed
    /// 5. Symlink filter (`allow_symlinks`) - skip symlinks unless allowed
    /// 6. Type filter (`types`) - must match file type
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
        if let Some(dir) = &self.dir {
            files.retain(|f| f.starts_with(dir));
            if files.is_empty() {
                debug!("{self}: no files in {dir}");
            }
            // Don't strip the dir prefix here - it causes issues when steps have different working directories
            // The path stripping should only happen in the command execution context via tera templates
        }
        if let Some(pattern) = &self.glob {
            // Use get_pattern_matches consistently for both globs and regex
            files = glob::get_pattern_matches(pattern, &files, self.dir.as_deref())?;
        }
        if let Some(pattern) = &self.exclude {
            // Use get_pattern_matches consistently for excludes too
            let excluded: HashSet<_> =
                glob::get_pattern_matches(pattern, &files, self.dir.as_deref())?
                    .into_iter()
                    .collect();
            files.retain(|f| !excluded.contains(f));
        }

        // Filter out binary files unless allow_binary is true
        if !self.allow_binary {
            files.retain(|f| {
                // Keep file if we can't determine if it's binary (might be deleted/renamed)
                // or if it's definitely not binary
                is_binary_file(f).map(|is_bin| !is_bin).unwrap_or(true)
            });
        }

        // Filter out symbolic links unless allow_symlinks is true
        if !self.allow_symlinks {
            files.retain(|f| {
                // Keep file if we can't determine if it's a symlink (might be deleted/renamed)
                // or if it's definitely not a symlink
                is_symlink_file(f)
                    .map(|is_symlink| !is_symlink)
                    .unwrap_or(true)
            });
        }

        // Filter by file types if specified
        if let Some(types) = &self.types {
            files.retain(|f| crate::file_type::matches_types(f, types));
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
        let mut dirs = files.iter().filter_map(|f| f.parent()).collect_vec();
        let mut workspaces: IndexSet<PathBuf> = Default::default();
        while let Some(dir) = dirs.pop() {
            if let Some(workspace) = xx::file::find_up(dir, &[workspace_indicator]) {
                workspaces.insert(workspace);
            }
        }
        Ok(Some(workspaces))
    }
}
