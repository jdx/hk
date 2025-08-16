//! Error types and result helpers
//!
//! This module defines the error types used throughout the xx library.
//! All errors include additional context to help diagnose issues.
//!
//! ## Error Types
//!
//! - `FileError` - File operations with path context
//! - `GitError` - Git operations with repository path
//! - `ProcessError` - Process execution with command context
//! - Additional feature-specific errors when features are enabled
//!
//! ## Usage
//!
//! The `XXResult<T>` type alias is provided for convenience:
//!
//! ```rust
//! use xx::XXResult;
//!
//! fn my_function() -> XXResult<String> {
//!     xx::file::read_to_string("config.toml")
//! }
//! ```

use std::path::PathBuf;

use miette::Diagnostic;
use thiserror::Error;

/// Main error type for the xx library
#[derive(Error, Diagnostic, Debug)]
pub enum XXError {
    #[error("{0}")]
    Error(String),

    #[error("{0}\nFile: {1}")]
    #[diagnostic(code(xx::file), url(docsrs))]
    FileError(std::io::Error, PathBuf),

    #[error("{0}\nGit: {1}")]
    #[diagnostic(code(xx::git), url(docsrs))]
    GitError(std::io::Error, PathBuf),

    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::process), url(docsrs))]
    ProcessError(std::io::Error, String),

    #[cfg(any(
        feature = "archive_untar_gzip",
        feature = "archive_untar_bzip2",
        feature = "archive_untar_xz",
        feature = "archive_unzip",
        feature = "archive_ungz"
    ))]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::archive), url(docsrs))]
    ArchiveIOError(std::io::Error, PathBuf),

    #[cfg(feature = "archive_unzip")]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::archive), url(docsrs))]
    ArchiveZipError(zip::result::ZipError, PathBuf),

    #[cfg(feature = "glob")]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::glob), url(docsrs))]
    GlobwalkError(globwalk::GlobError, PathBuf),

    #[cfg(feature = "http")]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::http), url(docsrs))]
    HTTPError(reqwest::Error, String),

    #[cfg(feature = "fslock")]
    #[error("{0}\n{1}")]
    #[diagnostic(code(xx::fslock), url(docsrs))]
    FSLockError(fslock::Error, String),
}

/// A specialized Result type for xx operations
///
/// This type alias is used throughout the xx library for functions that may return an error.
/// It's equivalent to `Result<T, XXError>`.
///
/// ## Example
///
/// ```rust
/// use xx::XXResult;
///
/// fn read_config() -> XXResult<String> {
///     xx::file::read_to_string("config.toml")
/// }
/// ```
pub type XXResult<T> = Result<T, XXError>;

/// Create an XXError with a formatted message
///
/// ## Example
///
/// ```rust
/// use xx::error;
///
/// let err = error!("Failed to process file: {}", "test.txt");
/// ```
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::error::XXError::Error(format!($($arg)*))
    };
}

/// Return early with an XXError
///
/// This macro is equivalent to `return Err(error!(...))`.
///
/// ## Example
///
/// ```rust,no_run
/// use xx::bail;
///
/// fn validate(value: i32) -> xx::XXResult<()> {
///     if value < 0 {
///         bail!("Value must be non-negative, got {}", value);
///     }
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::error!($($arg)*));
    };
}
