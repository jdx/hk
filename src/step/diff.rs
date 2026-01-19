//! Applying unified diffs directly to files.
//!
//! When a step has `check_diff` configured, instead of running the fixer command,
//! hk can apply the diff output directly using `git apply`. This is often faster
//! than running the fixer, especially for tools that are slow to start.

use crate::Result;
use std::io::Write;

use super::types::Step;

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
    /// # Arguments
    ///
    /// * `stdout` - The unified diff output from the check_diff command
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Diff was applied successfully
    /// * `Ok(false)` - Diff application failed (caller should fall back to fixer)
    /// * `Err(_)` - Unexpected error
    pub(crate) fn apply_diff_output(&self, stdout: &str) -> Result<bool> {
        if stdout.trim().is_empty() {
            debug!("{}: no diff content to apply", self.name);
            return Ok(false);
        }
        let diff_content = stdout;

        // Detect if this diff uses a/ and b/ prefixes (git-style)
        // Use -p1 to strip prefixes if present, -p0 otherwise
        let mut has_a_prefix = false;
        let mut has_b_prefix = false;
        for line in stdout.lines() {
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

        if let Some(dir) = &self.dir {
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
        if let Some(stdin) = child.stdin.as_mut() {
            if let Err(e) = stdin.write_all(diff_content.as_bytes()) {
                warn!("{}: failed to write diff to git apply: {}", self.name, e);
                return Ok(false);
            }
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
