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
//! - [`dir`] - Resolving a step's working directory
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
mod command;
mod diff;
mod dir;
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
pub(crate) use command::argv_runner;
pub use expr_env::{EXPR_CTX, eval_condition};
pub use shell::ShellType;
pub(crate) use types::RenderedCommand;
#[cfg(test)]
pub(crate) use types::{ArgvCommand, Command};
pub use types::{
    CommandEffect, CommandPrefix, DiagnosticFormat, FileSelector, OutputSummary, Pattern, RunType,
    Script, Step,
};

// Re-export for potential external use (currently only used internally)
#[allow(unused_imports)]
pub use filtering::{is_binary_file, is_symlink_file};

/// Normalize tool-specific quirks in unified diff headers so a diff can be
/// attributed to files and handed to `git apply`.
///
/// Two Go toolchain conventions need this:
/// - gofmt writes `--- file.go.orig` against a plain `+++ file.go`.
/// - `go fix -diff` labels both sides: `--- file.go (old)` / `+++ file.go (new)`.
///
/// The `(old)`/`(new)` form is only rewritten when both sides carry their label,
/// so a file genuinely named `foo (old)` is left alone.
pub(crate) fn normalize_diff_paths(diff: &str) -> String {
    let mut result: Vec<String> = Vec::new();
    let mut lines = diff.lines().peekable();
    while let Some(line) = lines.next() {
        if let Some(after_prefix) = line.strip_prefix("--- ")
            && let Some(next) = lines.peek()
            && next.starts_with("+++ ")
        {
            // `go fix -diff`: both sides labelled.
            if let Some(old_path) = after_prefix.strip_suffix(" (old)")
                && let Some(new_path) = next
                    .strip_prefix("+++ ")
                    .and_then(|p| p.strip_suffix(" (new)"))
            {
                result.push(format!("--- {old_path}"));
                result.push(format!("+++ {new_path}"));
                lines.next();
                continue;
            }
            // gofmt: ".orig" on the "---" line only.
            if !next.contains(".orig") {
                // Extract path portion (before any tab-separated timestamp)
                let (path, rest) = after_prefix.split_once('\t').unwrap_or((after_prefix, ""));
                if let Some(stripped) = path.strip_suffix(".orig") {
                    if rest.is_empty() {
                        result.push(format!("--- {stripped}"));
                    } else {
                        result.push(format!("--- {stripped}\t{rest}"));
                    }
                    continue;
                }
            }
        }
        result.push(line.to_string());
    }
    result.join("\n") + "\n"
}

#[cfg(test)]
mod normalize_diff_paths_tests {
    use super::normalize_diff_paths;

    #[test]
    fn strips_go_fix_old_new_labels() {
        let diff = "--- /w/main.go (old)\n+++ /w/main.go (new)\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- /w/main.go\n+++ /w/main.go\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn strips_gofmt_orig_suffix() {
        let diff = "--- main.go.orig\n+++ main.go\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- main.go\n+++ main.go\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn leaves_plain_headers_alone() {
        let diff = "--- a/main.go\n+++ b/main.go\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
    }

    #[test]
    fn keeps_a_file_actually_named_old_when_the_pair_is_unlabelled() {
        // Only the "---" side carries "(old)", so this is a real filename.
        let diff = "--- notes (old)\n+++ notes (old)\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
    }
}
