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
pub(crate) use job_builder::SharedBatchJobs;
pub use shell::ShellType;
pub(crate) use types::RenderedCommand;
#[cfg(test)]
pub(crate) use types::{ArgvCommand, Command};
pub use types::{
    CommandEffect, CommandPrefix, DiagnosticFormat, FileSelector, OutputSummary, Pattern, RunType,
    Script, Step,
};

pub(crate) use filtering::cache_symlink_check;
// Re-export for potential external use (currently only used internally)
#[allow(unused_imports)]
pub use filtering::{is_binary_file, is_symlink_file};

/// Split `line` (from `str::split_inclusive('\n')`) into its content and its
/// line terminator.
///
/// Diffs are processed line by line with their terminators kept: `str::lines`
/// drops a trailing `\r`, which would turn a patch for a file with CRLF line
/// endings into one that no longer matches the file.
pub(crate) fn split_line_ending(line: &str) -> (&str, &str) {
    let content = line.strip_suffix('\n').unwrap_or(line);
    let content = content.strip_suffix('\r').unwrap_or(content);
    (content, &line[content.len()..])
}

/// Normalize tool-specific quirks in unified diff headers so a diff can be
/// attributed to files and handed to `git apply`.
///
/// - gofmt writes `--- file.go.orig` against a plain `+++ file.go`.
/// - `go fix -diff` labels both sides: `--- file.go (old)` / `+++ file.go (new)`.
/// - Some tools prefix each side with its own directory name instead of git's
///   `a/` and `b/`, such as `--- current/go.mod` / `+++ tidy/go.mod` from
///   `go mod tidy -diff`. When the two sides differ only in their first path
///   component, it is rewritten to `a/` and `b/`, which hk strips.
///
/// The `(old)`/`(new)` form is only rewritten when both sides carry their label,
/// so a file genuinely named `foo (old)` is left alone. Lines other than headers
/// are kept byte for byte, including carriage returns.
pub(crate) fn normalize_diff_paths(diff: &str) -> String {
    let mut result = String::with_capacity(diff.len() + 1);
    let mut lines = diff.split_inclusive('\n').peekable();
    while let Some(line) = lines.next() {
        let (content, ending) = split_line_ending(line);
        if let Some(after_prefix) = content.strip_prefix("--- ")
            && let Some(next) = lines.peek()
            && let (next_content, next_ending) = split_line_ending(next)
            && let Some(next_path) = next_content.strip_prefix("+++ ")
        {
            // `go fix -diff`: both sides labelled.
            if let Some(old_path) = after_prefix.strip_suffix(" (old)")
                && let Some(new_path) = next_path.strip_suffix(" (new)")
            {
                result.push_str(&format!(
                    "--- {old_path}{ending}+++ {new_path}{next_ending}"
                ));
                lines.next();
                continue;
            }
            // Each side under its own top-level directory name.
            if let Some((old_path, new_path)) = relabel_sides(after_prefix, next_path) {
                result.push_str(&format!(
                    "--- {old_path}{ending}+++ {new_path}{next_ending}"
                ));
                lines.next();
                continue;
            }
            // gofmt: ".orig" on the "---" line only.
            if !next_path.contains(".orig") {
                // Extract path portion (before any tab-separated timestamp)
                let (path, rest) = after_prefix.split_once('\t').unwrap_or((after_prefix, ""));
                if let Some(stripped) = path.strip_suffix(".orig") {
                    if rest.is_empty() {
                        result.push_str(&format!("--- {stripped}{ending}"));
                    } else {
                        result.push_str(&format!("--- {stripped}\t{rest}{ending}"));
                    }
                    continue;
                }
            }
        }
        result.push_str(line);
    }
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// `a/<path>` and `b/<path>` for header paths `<old>/<path>` and `<new>/<path>`
/// whose first components differ, or `None` for any other pair. Git's own
/// `a/`/`b/` pair comes back unchanged. Tab-separated timestamps are kept.
fn relabel_sides(old: &str, new: &str) -> Option<(String, String)> {
    let (old_path, old_tail) = old
        .split_once('\t')
        .map_or((old, None), |(p, t)| (p, Some(t)));
    let (new_path, new_tail) = new
        .split_once('\t')
        .map_or((new, None), |(p, t)| (p, Some(t)));
    let (old_dir, old_rest) = old_path.split_once('/')?;
    let (new_dir, new_rest) = new_path.split_once('/')?;
    if old_dir.is_empty() || new_dir.is_empty() || old_dir == new_dir {
        return None;
    }
    if old_rest.is_empty() || old_rest != new_rest || old_rest.starts_with('/') {
        return None;
    }
    let with_tail = |path: String, tail: Option<&str>| match tail {
        Some(tail) => format!("{path}\t{tail}"),
        None => path,
    };
    Some((
        with_tail(format!("a/{old_rest}"), old_tail),
        with_tail(format!("b/{new_rest}"), new_tail),
    ))
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
    fn relabels_sides_under_their_own_directories() {
        // `go mod tidy -diff`
        let diff = "diff current/go.mod tidy/go.mod\n--- current/go.mod\n+++ tidy/go.mod\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "diff current/go.mod tidy/go.mod\n--- a/go.mod\n+++ b/go.mod\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn relabels_sides_and_keeps_timestamps() {
        let diff =
            "--- old/tf/main.tf\t2025-01-01\n+++ new/tf/main.tf\t2025-01-02\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- a/tf/main.tf\t2025-01-01\n+++ b/tf/main.tf\t2025-01-02\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn leaves_sides_in_the_same_directory_alone() {
        let diff = "--- src/main.go\n+++ src/main.go\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
    }

    #[test]
    fn leaves_different_files_alone() {
        let diff = "--- old/a.go\n+++ new/b.go\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
    }

    #[test]
    fn keeps_carriage_returns() {
        let diff = "--- a/f.txt\n+++ b/f.txt\n@@ -1,2 +1,2 @@\n-one\r\n+one\n two\n";
        assert_eq!(normalize_diff_paths(diff), diff);
        let diff = "--- f.go.orig\n+++ f.go\n@@ -1 +1 @@\n-one\r\n+one\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- f.go\n+++ f.go\n@@ -1 +1 @@\n-one\r\n+one\n"
        );
    }

    #[test]
    fn keeps_a_file_actually_named_old_when_the_pair_is_unlabelled() {
        // Only the "---" side carries "(old)", so this is a real filename.
        let diff = "--- notes (old)\n+++ notes (old)\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
    }
}
