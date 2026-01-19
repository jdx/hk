//! Step configuration and execution.
//!
//! This module provides the core step functionality for hk. A step represents
//! a single linting or formatting task that operates on files.
//!
//! # Module Organization
//!
//! - [`types`] - Core type definitions (Step, Pattern, Script, RunType, OutputSummary)
//! - [`shell`] - Shell type detection and quoting utilities
//! - [`filtering`] - File filtering, binary/symlink detection, profile handling
//! - [`batching`] - ARG_MAX handling and job batching
//! - [`job_builder`] - Step job creation
//! - [`execution`] - Async job orchestration
//! - [`runner`] - Single job execution
//! - [`check_parsing`] - Parsing check_list_files and check_diff output
//! - [`diff`] - Applying unified diffs directly
//! - [`output`] - Output capture and fix suggestions
//! - [`progress`] - Progress bar management
//! - [`expr_env`] - Expression evaluation for conditions
//!
//! # Usage
//!
//! Steps are typically created from configuration (hk.pkl) and executed via hooks:
//!
//! ```ignore
//! // Steps are defined in hk.pkl
//! ["eslint"] {
//!     glob = "*.{js,ts}"
//!     check = "eslint {{files}}"
//!     fix = "eslint --fix {{files}}"
//! }
//! ```

mod batching;
mod check_parsing;
mod diff;
mod execution;
mod expr_env;
mod filtering;
mod job_builder;
mod output;
mod progress;
mod runner;
mod shell;
mod types;

// Re-export public API
pub use expr_env::EXPR_CTX;
pub use shell::ShellType;
pub use types::{OutputSummary, Pattern, RunType, Script, Step};

// Re-export for potential external use (currently only used internally)
#[allow(unused_imports)]
pub use filtering::{is_binary_file, is_symlink_file};
