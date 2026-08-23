//! Applying unified diffs directly to files.
//!
//! When a step has `check_diff` configured, instead of running the fixer command,
//! hk can apply the diff output directly using `git apply`. This is often faster
//! than running the fixer, especially for tools that are slow to start.

use crate::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::normalize_diff_paths;
use super::types::Step;

/// Rewrite absolute paths in diff headers to be relative to `base`.
///
/// `git apply` rejects absolute paths outright -- `--unsafe-paths` does not
/// change that, and `-p<n>` would need a strip depth that varies per checkout --
/// so a tool reporting absolute paths cannot be applied without this. `go fix
/// -diff` is one such tool.
///
/// Paths outside `base` are left as they are; `git apply` will reject them and
/// the caller falls back to running the fixer.
fn relativize_diff_paths(diff: &str, base: &Path) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in diff.lines() {
        let rewritten = ["--- ", "+++ "].into_iter().find_map(|prefix| {
            let rest = line.strip_prefix(prefix)?;
            // Keep any tab-separated timestamp attached to the path.
            let (path, tail) = match rest.split_once('\t') {
                Some((p, t)) => (p, Some(t)),
                None => (rest, None),
            };
            let rel = Path::new(path).strip_prefix(base).ok()?.to_str()?;
            Some(match tail {
                Some(t) => format!("{prefix}{rel}\t{t}"),
                None => format!("{prefix}{rel}"),
            })
        });
        out.push(rewritten.unwrap_or_else(|| line.to_string()));
    }
    out.join("\n") + "\n"
}

impl Step {
    /// Apply a unified diff directly to files using `git apply`.
    ///
    /// This provides a fast path for fixing files when `check_diff` is configured.
    /// Instead of running the potentially slow fixer command, the diff output
    /// can be applied directly.
    ///
    /// Automatically detects whether the diff uses `a/` and `b/` prefixes (git-style)
    /// and sets the appropriate strip level (`-p1` or `-p0`).
    ///
    /// Also handles Go-style diffs where the `---` line has a `.orig` suffix
    /// (e.g., `--- file.go.orig` instead of `--- file.go`).
    ///
    /// # Arguments
    ///
    /// * `stdout` - The unified diff output from the check_diff command
    /// * `dir` - The job's rendered working directory, or `None` for the repo root
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Diff was applied successfully
    /// * `Ok(false)` - Diff application failed (caller should fall back to fixer)
    /// * `Err(_)` - Unexpected error
    pub(crate) fn apply_diff_output(&self, stdout: &str, dir: Option<&str>) -> Result<bool> {
        if stdout.trim().is_empty() {
            debug!("{}: no diff content to apply", self.name);
            return Ok(false);
        }
        let diff_content = normalize_diff_paths(stdout);

        // Resolve against wherever `git apply` will run, so absolute paths
        // reported by the check command become paths git will accept.
        let base = PathBuf::from(dir.unwrap_or("."));
        let base = base.canonicalize().unwrap_or(base);
        let diff_content = relativize_diff_paths(&diff_content, &base);

        // Detect if this diff uses a/ and b/ prefixes (git-style)
        // Use -p1 to strip prefixes if present, -p0 otherwise
        let mut has_a_prefix = false;
        let mut has_b_prefix = false;
        for line in diff_content.lines() {
            if line.starts_with("--- a/") {
                has_a_prefix = true;
            } else if line.starts_with("+++ b/") {
                has_b_prefix = true;
            }
            if has_a_prefix && has_b_prefix {
                break;
            }
        }
        let strip_level = if has_a_prefix && has_b_prefix {
            "-p1"
        } else {
            "-p0"
        };

        // Use --whitespace=nowarn to avoid warnings about whitespace
        // Run in the step's directory if configured (same as check_diff command)
        let mut cmd = std::process::Command::new("git");
        cmd.args(["apply", strip_level, "--whitespace=nowarn", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }

        let result = cmd.spawn();

        let mut child = match result {
            Ok(c) => c,
            Err(e) => {
                warn!("{}: failed to spawn git apply: {}", self.name, e);
                return Ok(false);
            }
        };

        // Write diff to stdin
        if let Some(stdin) = child.stdin.as_mut()
            && let Err(e) = stdin.write_all(diff_content.as_bytes())
        {
            warn!("{}: failed to write diff to git apply: {}", self.name, e);
            return Ok(false);
        }

        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                warn!("{}: git apply failed to complete: {}", self.name, e);
                return Ok(false);
            }
        };

        if output.status.success() {
            debug!("{}: successfully applied diff", self.name);
            Ok(true)
        } else {
            let stderr_output = String::from_utf8_lossy(&output.stderr);
            debug!("{}: git apply failed: {}", self.name, stderr_output);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod relativize_diff_paths_tests {
    use super::relativize_diff_paths;
    use std::path::Path;

    #[test]
    fn rewrites_absolute_paths_under_base() {
        let diff = "--- /w/svc/main.go\n+++ /w/svc/main.go\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/w/svc")),
            "--- main.go\n+++ main.go\n@@ -1 +1 @@\n-a\n+b\n"
        );
    }

    #[test]
    fn leaves_paths_outside_base_alone() {
        let diff = "--- /elsewhere/main.go\n+++ /elsewhere/main.go\n";
        assert_eq!(relativize_diff_paths(diff, Path::new("/w/svc")), diff);
    }

    #[test]
    fn leaves_relative_paths_alone() {
        let diff = "--- main.go\n+++ main.go\n";
        assert_eq!(relativize_diff_paths(diff, Path::new("/w/svc")), diff);
    }

    #[test]
    fn preserves_a_tab_separated_timestamp() {
        let diff = "--- /w/svc/main.go\t2025-01-01 12:00:00\n+++ /w/svc/main.go\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/w/svc")),
            "--- main.go\t2025-01-01 12:00:00\n+++ main.go\n"
        );
    }

    #[test]
    fn does_not_touch_diff_body_lines() {
        let diff = "--- /w/svc/a.go\n+++ /w/svc/a.go\n@@ -1 +1 @@\n---- not a header\n";
        assert_eq!(
            relativize_diff_paths(diff, Path::new("/w/svc")),
            "--- a.go\n+++ a.go\n@@ -1 +1 @@\n---- not a header\n"
        );
    }
}
