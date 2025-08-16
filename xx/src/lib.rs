//! # xx
//!
//! A collection of useful Rust macros and small utility functions to make common tasks easier.
//!
//! This library provides enhanced alternatives to standard library functions with better error
//! messages, additional convenience methods, and commonly needed functionality that's missing
//! from the standard library.
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! xx = "2.1"
//! ```
//!
//! ## Core Features
//!
//! - **Enhanced file operations** - File I/O with better error messages and automatic parent directory creation
//! - **Process execution** - Convenient process spawning with builder pattern
//! - **Git operations** - High-level git repository management
//! - **Error handling** - Improved error types with context
//!
//! ## Optional Features
//!
//! Enable additional functionality by adding features to your dependency:
//!
//! ```toml
//! [dependencies]
//! xx = { version = "2.1", features = ["archive", "glob", "hash"] }
//! ```
//!
//! Available features:
//! - `archive` - Archive extraction (tar.gz, tar.bz2, tar.xz, zip)
//! - `glob` - File globbing support
//! - `hash` - SHA256 hashing utilities
//! - `http` - HTTP client functionality
//! - `fslock` - File system locking
//!
//! ## Examples
//!
//! ### File Operations
//!
//! ```rust,no_run
//! use xx::file;
//!
//! # fn main() -> xx::XXResult<()> {
//! // Read file with enhanced error messages
//! let content = file::read_to_string("config.toml")?;
//!
//! // Write file, creating parent directories automatically
//! file::write("output/data.txt", "Hello, world!")?;
//!
//! // Create directory and all parents
//! file::mkdirp("path/to/deep/directory")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Process Execution
//!
//! ```rust,no_run
//! use xx::process;
//!
//! # fn main() -> xx::XXResult<()> {
//! // Run shell command
//! let output = process::sh("ls -la")?;
//!
//! // Build and run commands with builder pattern
//! let result = process::cmd("git", &["status"]).read()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Git Operations
//!
//! ```rust,no_run
//! use xx::git::{Git, CloneOptions};
//!
//! # fn main() -> xx::XXResult<()> {
//! // Clone a repository
//! let options = CloneOptions::default().branch("main");
//! let repo = xx::git::clone("https://github.com/user/repo", "/tmp/repo", &options)?;
//!
//! // Work with existing repository
//! let git = Git::new("/path/to/repo".into());
//! let branch = git.current_branch()?;
//! let sha = git.current_sha()?;
//! # Ok(())
//! # }
//! ```

#![allow(unused_attributes)]

#[macro_use]
extern crate log;

#[macro_use]
pub use error::{XXError, XXResult};

/// Context management utilities
pub mod context;
/// Environment variable parsing utilities
pub mod env;
/// Error types and result helpers
pub mod error;
/// Enhanced file operations with better error handling
pub mod file;
/// File system locking functionality (requires `fslock` feature)
#[cfg(feature = "fslock")]
pub mod fslock;
/// Git repository operations
pub mod git;
/// Process execution utilities
pub mod process;
mod regex;

/// Archive extraction utilities (requires one of the archive features)
#[cfg(any(
    feature = "archive_untar_gzip",
    feature = "archive_untar_bzip2",
    feature = "archive_untar_xz",
    feature = "archive_unzip",
    feature = "archive_ungz",
))]
pub mod archive;

/// SHA256 hashing utilities (requires `hash` feature)
#[cfg(feature = "hash")]
pub mod hash;
/// HTTP client functionality (requires `http` feature)
#[cfg(feature = "http")]
pub mod http;

#[cfg(test)]
pub mod test;
