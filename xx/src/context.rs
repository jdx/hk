//! Context management utilities
//!
//! This module provides utilities for managing global context, particularly
//! for handling path resolution with a configurable root directory.
//!
//! ## Load Root
//!
//! The load root is a global path prefix that can be used to resolve relative paths.
//! This is useful when your application needs to work with files relative to a
//! specific directory, regardless of the current working directory.
//!
//! ## Example
//!
//! ```rust
//! use xx::context;
//! use std::path::PathBuf;
//!
//! // Set a load root directory
//! context::set_load_root("/app/data");
//!
//! // Relative paths will be resolved against the load root
//! let config_path = context::prepend_load_root("config.toml");
//! assert_eq!(config_path, PathBuf::from("/app/data/config.toml"));
//!
//! // Absolute paths are returned unchanged
//! let abs_path = context::prepend_load_root("/etc/config.toml");
//! assert_eq!(abs_path, PathBuf::from("/etc/config.toml"));
//! ```

use std::path::{Path, PathBuf};
use std::sync::Mutex;

static LOAD_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Get the current load root directory
///
/// Returns the configured load root, or an empty path if none is set.
pub fn get_load_root() -> PathBuf {
    LOAD_ROOT.lock().unwrap().clone().unwrap_or_default()
}

/// Set the load root directory
///
/// This directory will be used as a prefix for relative paths in `prepend_load_root`.
pub fn set_load_root<P: Into<PathBuf>>(root: P) {
    *LOAD_ROOT.lock().unwrap() = Some(root.into());
}

/// Prepend the load root to a relative path
///
/// If the input path is relative, it will be joined with the load root.
/// If the input path is absolute, it will be returned unchanged.
pub fn prepend_load_root<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    match path.is_relative() {
        true => get_load_root().join(path),
        false => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use test_log::test;

    use super::*;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_set_load_root() {
        let _t = TEST_MUTEX.lock().unwrap();
        set_load_root(PathBuf::from("/foo/bar"));
        assert_eq!(get_load_root(), PathBuf::from("/foo/bar"));
    }

    #[test]
    fn test_prepend_load_root() {
        let _t = TEST_MUTEX.lock().unwrap();
        set_load_root(PathBuf::from("/foo/bar"));
        assert_eq!(
            prepend_load_root(Path::new("baz")),
            PathBuf::from("/foo/bar/baz")
        );
        assert_eq!(prepend_load_root(Path::new("/baz")), PathBuf::from("/baz"));
    }
}
