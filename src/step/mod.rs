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

/// The lines of a unified diff, keeping their terminators, each paired with
/// whether it is in a hunk body.
///
/// A hunk's length comes from its `@@ -a,b +c,d @@` header. Inside one, a line
/// starting with `--- ` or `+++ ` is removed or added content (such as a
/// removed `-- x`), not a file header, so nothing that looks for headers may
/// treat it as one.
pub(crate) fn diff_lines(diff: &str) -> Vec<(&str, bool)> {
    let mut lines = Vec::new();
    let (mut old_left, mut new_left) = (0usize, 0usize);
    for line in diff.split_inclusive('\n') {
        if old_left > 0 || new_left > 0 {
            match line.as_bytes().first() {
                Some(b'-') => old_left = old_left.saturating_sub(1),
                Some(b'+') => new_left = new_left.saturating_sub(1),
                Some(b'\\') => {}
                // Context, including an empty line some tools write for a
                // blank context line.
                _ => {
                    old_left = old_left.saturating_sub(1);
                    new_left = new_left.saturating_sub(1);
                }
            }
            lines.push((line, true));
            continue;
        }
        if let Some((old, new)) = hunk_lengths(split_line_ending(line).0) {
            (old_left, new_left) = (old, new);
        }
        lines.push((line, false));
    }
    lines
}

/// The old and new line counts of a `@@ -a[,b] +c[,d] @@` hunk header.
fn hunk_lengths(line: &str) -> Option<(usize, usize)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, _) = rest.split_once(" @@")?;
    let count = |range: &str| match range.split_once(',') {
        Some((_, count)) => count.parse().ok(),
        None => range.parse::<usize>().ok().map(|_| 1),
    };
    Some((count(old)?, count(new)?))
}

/// Normalize tool-specific quirks in unified diff headers so a diff can be
/// attributed to files and handed to `git apply`.
///
/// - gofmt writes `--- file.go.orig` against a plain `+++ file.go`.
/// - `go fix -diff` labels both sides: `--- file.go (old)` / `+++ file.go (new)`.
/// - isort labels them `--- file.py:before` / `+++ file.py:after`.
/// - `go mod tidy -diff` puts each side under a directory named for it,
///   `--- current/go.mod` / `+++ tidy/go.mod`, and `terraform fmt -diff` uses
///   `old/` and `new/`. These are rewritten to git's `a/` and `b/`.
///
/// The labelled forms are only rewritten when both sides carry their label,
/// so a file genuinely named `foo (old)` is left alone. Only file headers are
/// rewritten; hunk bodies are kept byte for byte, including carriage returns.
/// If some header pairs use git's `a/` and `b/` and others don't, the prefixes
/// are removed, so one strip level applies to the whole patch.
pub(crate) fn normalize_diff_paths(diff: &str) -> String {
    let lines = diff_lines(diff);
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    // The index in `out` of each header pair's `---` line.
    let mut headers = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let (line, in_hunk) = lines[i];
        let (content, ending) = split_line_ending(line);
        if !in_hunk
            && let Some(after_prefix) = content.strip_prefix("--- ")
            && let Some(&(next, false)) = lines.get(i + 1)
            && let (next_content, next_ending) = split_line_ending(next)
            && let Some(next_path) = next_content.strip_prefix("+++ ")
        {
            let (old_path, new_path) = strip_labels(after_prefix, next_path)
                .unwrap_or_else(|| (after_prefix.to_string(), next_path.to_string()));
            headers.push(out.len());
            out.push(format!("--- {old_path}{ending}"));
            out.push(format!("+++ {new_path}{next_ending}"));
            i += 2;
            continue;
        }
        out.push(line.to_string());
        i += 1;
    }
    let git_pairs: Vec<usize> = headers
        .iter()
        .copied()
        .filter(|&at| is_git_pair(&out[at][4..], &out[at + 1][4..]))
        .collect();
    // Without a full `a/` and `b/` pair, a `b/` on a created file may be a
    // real directory, so nothing is rewritten and the patch applies as is.
    let has_full_pair = headers
        .iter()
        .any(|&at| is_full_git_pair(&out[at][4..], &out[at + 1][4..]));
    if has_full_pair && git_pairs.len() < headers.len() {
        for at in git_pairs {
            // A created or deleted file has `/dev/null` on one side, which
            // keeps its path.
            if out[at][4..].starts_with("a/") {
                out[at].replace_range(4..6, "");
            }
            if out[at + 1][4..].starts_with("b/") {
                out[at + 1].replace_range(4..6, "");
            }
        }
    }
    let mut result = out.concat();
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// The `---` and `+++` sides of each file header pair outside hunk bodies,
/// without the markers or line terminators.
pub(crate) fn header_pairs(diff: &str) -> Vec<(&str, &str)> {
    let lines = diff_lines(diff);
    let mut pairs = Vec::new();
    for (i, &(line, in_hunk)) in lines.iter().enumerate() {
        if !in_hunk
            && let Some(old) = split_line_ending(line).0.strip_prefix("--- ")
            && let Some(&(next, false)) = lines.get(i + 1)
            && let Some(new) = split_line_ending(next).0.strip_prefix("+++ ")
        {
            pairs.push((old, new));
        }
    }
    pairs
}

/// A header side's path, without any tab-separated timestamp.
pub(crate) fn header_path(side: &str) -> &str {
    side.split_once('\t').map_or(side, |(path, _)| path).trim()
}

/// Whether a header pair uses git's `a/` and `b/` prefixes. A created or
/// deleted file has `/dev/null` on one side.
pub(crate) fn is_git_pair(old: &str, new: &str) -> bool {
    let (old, new) = (header_path(old), header_path(new));
    let old_prefixed = old.starts_with("a/");
    let new_prefixed = new.starts_with("b/");
    (old_prefixed || old == "/dev/null")
        && (new_prefixed || new == "/dev/null")
        && (old_prefixed || new_prefixed)
}

/// Whether a header pair carries both of git's prefixes, `a/` and `b/`.
fn is_full_git_pair(old: &str, new: &str) -> bool {
    header_path(old).starts_with("a/") && header_path(new).starts_with("b/")
}

/// Whether `git apply` must strip git's `a/` and `b/` prefixes (`-p1`) from
/// a diff's paths. That takes a pair with both prefixes, or a `diff --git`
/// line while every pair still uses git's paths: a created file's `b/new`
/// alone could be a real directory (see [`creations_use_git_prefixes`]).
/// [`normalize_diff_paths`] makes every pair agree, and when it removes the
/// prefixes from a mixed patch, a leftover `diff --git` line no longer counts.
pub(crate) fn uses_git_prefixes(diff: &str) -> bool {
    let pairs = header_pairs(diff);
    if pairs.iter().any(|&(old, new)| is_full_git_pair(old, new)) {
        return true;
    }
    let git_header = diff_lines(diff)
        .into_iter()
        .any(|(line, in_hunk)| !in_hunk && line.starts_with("diff --git a/"));
    git_header && pairs.iter().all(|&(old, new)| is_git_pair(old, new))
}

/// Whether a patch applied in `base` needs git's `a/` and `b/` stripped
/// (`-p1`). Applying a patch and listing the files it changes both use this,
/// so they agree on the paths.
pub(crate) fn strips_git_prefixes(diff: &str, base: &std::path::Path) -> bool {
    uses_git_prefixes(diff) || creations_use_git_prefixes(diff, base)
}

/// For a patch whose every pair creates or deletes a file with a prefix,
/// whether those prefixes are git's, judged by what exists under `base`,
/// where the patch applies: a deleted `a/x` is git's when `x` exists and
/// `a/x` doesn't, and a created `b/x` is git's unless `b` is a directory. A
/// patch that also has an unprefixed pair is not, since stripping a
/// component would move that pair's path too.
pub(crate) fn creations_use_git_prefixes(diff: &str, base: &std::path::Path) -> bool {
    let pairs: Vec<(&str, &str)> = header_pairs(diff)
        .into_iter()
        .map(|(old, new)| (header_path(old), header_path(new)))
        .collect();
    !pairs.is_empty()
        && pairs.iter().all(|&(old, new)| is_git_pair(old, new))
        && pairs.iter().all(|&(old, new)| {
            if let Some(rest) = old.strip_prefix("a/") {
                !base.join(old).exists() && base.join(rest).exists()
            } else {
                // `is_git_pair` means the other side is `/dev/null`.
                new.strip_prefix("b/").is_some() && !base.join("b").is_dir()
            }
        })
}

/// The header paths with a tool's side labels removed, or `None` if the pair
/// carries none that hk knows.
fn strip_labels(old: &str, new: &str) -> Option<(String, String)> {
    // `go fix -diff`: both sides labelled.
    if let Some(old_path) = old.strip_suffix(" (old)")
        && let Some(new_path) = new.strip_suffix(" (new)")
    {
        return Some((old_path.to_string(), new_path.to_string()));
    }
    // isort: `path:before` and `path:after`, before any timestamp.
    if let Some(pair) = strip_side_suffixes(old, new) {
        return Some(pair);
    }
    if let Some(pair) = relabel_sides(old, new) {
        return Some(pair);
    }
    // gofmt: ".orig" on the "---" line only.
    if !new.contains(".orig") {
        let (path, rest) = old.split_once('\t').unwrap_or((old, ""));
        if let Some(stripped) = path.strip_suffix(".orig") {
            let old = if rest.is_empty() {
                stripped.to_string()
            } else {
                format!("{stripped}\t{rest}")
            };
            return Some((old, new.to_string()));
        }
    }
    None
}

/// Header paths without isort's `:before`/`:after` labels, or `None` unless
/// both sides carry theirs. Tab-separated timestamps are kept.
fn strip_side_suffixes(old: &str, new: &str) -> Option<(String, String)> {
    let strip = |side: &str, label: &str| -> Option<String> {
        let (path, tail) = side
            .split_once('\t')
            .map_or((side, None), |(p, t)| (p, Some(t)));
        let path = path.strip_suffix(label)?;
        Some(match tail {
            Some(tail) => format!("{path}\t{tail}"),
            None => path.to_string(),
        })
    };
    Some((strip(old, ":before")?, strip(new, ":after")?))
}

/// Directory names that tools put each side of a diff under, instead of git's
/// `a/` and `b/`. Only these are relabelled: a diff between two real files
/// such as `docs/x` and `site/x` names both paths and must keep them.
const SIDE_DIRECTORIES: &[(&str, &str)] = &[
    // `go mod tidy -diff`
    ("current", "tidy"),
    // `terraform fmt -diff`, `tofu fmt -diff`
    ("old", "new"),
];

/// `a/<path>` and `b/<path>` for header paths `<old>/<path>` and `<new>/<path>`
/// under a known pair of side directories, or `None` for any other pair.
/// Tab-separated timestamps are kept.
fn relabel_sides(old: &str, new: &str) -> Option<(String, String)> {
    let (old_path, old_tail) = old
        .split_once('\t')
        .map_or((old, None), |(p, t)| (p, Some(t)));
    let (new_path, new_tail) = new
        .split_once('\t')
        .map_or((new, None), |(p, t)| (p, Some(t)));
    let (old_dir, old_rest) = old_path.split_once('/')?;
    let (new_dir, new_rest) = new_path.split_once('/')?;
    if !SIDE_DIRECTORIES.contains(&(old_dir, new_dir)) {
        return None;
    }
    if old_rest.is_empty() || old_rest != new_rest || old_rest.starts_with('/') {
        return None;
    }
    // A side under a directory that exists names real files, not a label.
    if std::path::Path::new(old_dir).is_dir() || std::path::Path::new(new_dir).is_dir() {
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
    fn strips_isort_before_after_labels() {
        let diff = "--- /w/t.py:before\t2026-01-01 10:00:00\n+++ /w/t.py:after\t2026-01-01 10:00:01\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- /w/t.py\t2026-01-01 10:00:00\n+++ /w/t.py\t2026-01-01 10:00:01\n@@ -1 +1 @@\n-a\n+b\n"
        );
        // Only one side labelled: a file really named that way.
        let diff = "--- t.py:before\n+++ t.py:before\n@@ -1 +1 @@\n-a\n+b\n";
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
    fn leaves_real_directories_alone() {
        // `src/` and `pkl/` exist in this repository, so they aren't labels.
        let diff = "--- src/x\n+++ pkl/x\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
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
        // Two real directories: only known side labels are relabelled.
        let diff = "--- docs/x.md\n+++ site/x.md\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
    }

    #[test]
    fn leaves_hunk_lines_that_look_like_headers_alone() {
        // The file changes from "-- current/foo" to "++ tidy/foo".
        let diff = "--- a/f.txt\n+++ b/f.txt\n@@ -1,2 +1,2 @@\n--- current/foo\n+++ tidy/foo\n x\n";
        assert_eq!(normalize_diff_paths(diff), diff);
        let diff = "--- a/f.txt\n+++ b/f.txt\n@@ -1 +1 @@\n--- m.go (old)\n+++ m.go (new)\n";
        assert_eq!(normalize_diff_paths(diff), diff);
    }

    #[test]
    fn rewrites_headers_after_a_hunk_ends() {
        let diff = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n--- current/go.mod\n+++ tidy/go.mod\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n--- a/go.mod\n+++ b/go.mod\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn keeps_git_prefixes_when_a_patch_creates_or_deletes_files() {
        let diff = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n--- /dev/null\n+++ b/new\n@@ -0,0 +1 @@\n+n\n--- a/old\n+++ /dev/null\n@@ -1 +0,0 @@\n-o\n";
        assert_eq!(normalize_diff_paths(diff), diff);
        assert!(super::uses_git_prefixes(diff));
    }

    #[test]
    fn strips_created_file_prefixes_along_with_the_rest() {
        let diff = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n--- /dev/null\n+++ b/new\n@@ -0,0 +1 @@\n+n\n--- y\n+++ y\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- x\n+++ x\n@@ -1 +1 @@\n-a\n+b\n--- /dev/null\n+++ new\n@@ -0,0 +1 @@\n+n\n--- y\n+++ y\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn a_diff_git_line_means_git_prefixes() {
        let diff = "diff --git a/go.sum b/go.sum\nnew file mode 100644\n--- /dev/null\n+++ b/go.sum\n@@ -0,0 +1 @@\n+x\n";
        assert!(super::uses_git_prefixes(diff));
    }

    #[test]
    fn a_diff_git_line_does_not_count_once_prefixes_are_removed() {
        // A mixed patch loses its prefixes, so its `diff --git` line must not
        // bring back -p1, which would strip `x`'s real directory.
        let diff = "diff --git a/src/x b/src/x\n--- a/src/x\n+++ b/src/x\n@@ -1 +1 @@\n-a\n+b\n--- y\n+++ y\n@@ -1 +1 @@\n-a\n+b\n";
        let normalized = normalize_diff_paths(diff);
        assert!(normalized.contains("--- src/x\n+++ src/x\n"));
        assert!(!super::uses_git_prefixes(&normalized));
    }

    #[test]
    fn creations_and_deletions_are_judged_by_what_exists() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let create = "--- /dev/null\n+++ b/go.sum\n@@ -0,0 +1 @@\n+x\n";
        assert!(super::creations_use_git_prefixes(create, base));
        std::fs::create_dir(base.join("b")).unwrap();
        assert!(!super::creations_use_git_prefixes(create, base));

        // An unprefixed edit in the same patch would lose its first component.
        let mixed = "--- /dev/null\n+++ b/go.sum\n@@ -0,0 +1 @@\n+x\n--- src/y\n+++ src/y\n@@ -1 +1 @@\n-a\n+b\n";
        let other = tempfile::tempdir().unwrap();
        assert!(!super::creations_use_git_prefixes(mixed, other.path()));

        let delete = "--- a/old.txt\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n";
        std::fs::write(base.join("old.txt"), "x\n").unwrap();
        assert!(super::creations_use_git_prefixes(delete, base));
        std::fs::create_dir(base.join("a")).unwrap();
        std::fs::write(base.join("a/old.txt"), "x\n").unwrap();
        assert!(!super::creations_use_git_prefixes(delete, base));
    }

    #[test]
    fn leaves_a_created_file_under_a_real_b_directory_alone() {
        // No pair has both `a/` and `b/`, so `b/` may be a real directory.
        let diff =
            "--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1 @@\n+n\n--- y\n+++ y\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(normalize_diff_paths(diff), diff);
        assert!(!super::uses_git_prefixes(diff));
        let diff = "--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1 @@\n+n\n";
        assert!(!super::uses_git_prefixes(diff));
    }

    #[test]
    fn gives_every_header_pair_one_strip_level() {
        let diff = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n--- y\n+++ y\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            normalize_diff_paths(diff),
            "--- x\n+++ x\n@@ -1 +1 @@\n-a\n+b\n--- y\n+++ y\n@@ -1 +1 @@\n-a\n+b\n"
        );
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
