//! Core type definitions for step configuration.
//!
//! This module contains the fundamental types used to define and configure steps:
//! - [`Step`] - The main configuration struct for a linting/formatting step
//! - [`Pattern`] - File matching patterns (globs or regex)
//! - [`Script`] - Platform-specific command scripts
//! - [`RunType`] - Whether to run in check or fix mode
//! - [`OutputSummary`] - How to capture and display command output

use crate::step_test::StepTest;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};
use std::{fmt, fmt::Display, path::PathBuf, str::FromStr};

/// A file matching pattern that can be either glob patterns or a regex.
///
/// Patterns are used to filter which files a step should operate on.
///
/// # Variants
///
/// * `Regex` - A regular expression pattern for complex matching
/// * `Globs` - One or more glob patterns (e.g., `*.rs`, `**/*.ts`)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Pattern {
    /// A regex pattern with explicit type marker
    Regex {
        #[serde(skip_serializing)]
        _type: String,
        /// The regex pattern string
        pattern: String,
    },
    /// One or more glob patterns
    Globs(Vec<String>),
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        use serde_json::Value;

        let value = Value::deserialize(deserializer)?;

        // Check if it's a regex object with _type field
        if let Value::Object(ref map) = value
            && let Some(Value::String(type_str)) = map.get("_type")
            && type_str == "regex"
            && let Some(Value::String(pattern)) = map.get("pattern")
        {
            return Ok(Pattern::Regex {
                _type: "regex".to_string(),
                pattern: pattern.clone(),
            });
        }

        // Try to deserialize as a string
        if let Value::String(s) = value {
            return Ok(Pattern::Globs(vec![s]));
        }

        // Try to deserialize as array of strings
        if let Value::Array(arr) = value {
            let globs: Result<Vec<String>, _> = arr
                .into_iter()
                .map(|v| {
                    if let Value::String(s) = v {
                        Ok(s)
                    } else {
                        Err(D::Error::custom("array elements must be strings"))
                    }
                })
                .collect();
            return Ok(Pattern::Globs(globs?));
        }

        Err(D::Error::custom(
            "expected regex object, string, or array of strings",
        ))
    }
}

/// A step configuration that defines a linting or formatting task.
///
/// Steps are the core building blocks of hk. Each step defines:
/// - What files to operate on (via globs, types, excludes)
/// - What commands to run (check, fix, check_diff, check_list_files)
/// - How to run them (shell, environment, working directory)
/// - Dependencies and execution constraints
///
/// # Example (in hk.pkl)
///
/// ```pkl
/// ["eslint"] {
///     glob = "*.{js,ts}"
///     check = "eslint {{files}}"
///     fix = "eslint --fix {{files}}"
/// }
/// ```
#[serde_as]
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct Step {
    /// Internal type marker (used by Pkl)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,

    /// Category for documentation grouping (e.g., "JavaScript/TypeScript", "Python", "Rust")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Human-readable description of the step for documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The step name (set during initialization)
    #[serde(default)]
    pub name: String,

    /// Profiles that enable/disable this step (prefix with `!` to disable)
    pub profiles: Option<Vec<String>>,

    /// File matching pattern (globs or regex)
    #[serde(default)]
    pub glob: Option<Pattern>,

    /// File types to match (e.g., `["rust", "toml"]`)
    #[serde(default)]
    pub types: Option<Vec<String>>,

    /// Whether this step requires interactive terminal input
    #[serde(default)]
    pub interactive: bool,

    /// Content to pipe to the command's stdin
    pub stdin: Option<String>,

    /// Steps that must complete before this one runs
    pub depends: Vec<String>,

    /// Custom shell to use (default: `sh -o errexit -c`)
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub shell: Option<Script>,

    /// Command to check for issues (exit non-zero if issues found)
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check: Option<Script>,

    /// Command that outputs a list of files needing fixes (one per line)
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check_list_files: Option<Script>,

    /// Command that outputs a unified diff of needed changes
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check_diff: Option<Script>,

    /// Command to fix issues
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub fix: Option<Script>,

    /// File that indicates workspace roots (e.g., `Cargo.toml` for Rust)
    pub workspace_indicator: Option<String>,

    /// Prefix to prepend to all commands
    pub prefix: Option<String>,

    /// Working directory for commands (relative to repo root)
    pub dir: Option<String>,

    /// Expression that must evaluate to true for step job to run
    #[serde(rename = "condition")]
    pub job_condition: Option<String>,

    /// Expression that must evaluate to true for step to run
    pub step_condition: Option<String>,

    /// Run check command before fix to identify files needing changes
    #[serde(default)]
    pub check_first: bool,

    /// Split files across multiple parallel jobs
    #[serde(default)]
    pub batch: bool,

    /// Allow overwriting files being processed by other steps
    #[serde(default)]
    pub stomp: bool,

    /// Environment variables to set
    pub env: IndexMap<String, String>,

    /// Glob patterns for files to stage after fixing
    pub stage: Option<Vec<String>>,

    /// Patterns to exclude from matching
    pub exclude: Option<Pattern>,

    /// Run this step alone (not in parallel with others)
    #[serde(default)]
    pub exclusive: bool,

    /// Whether to include binary files (default: false)
    #[serde(default)]
    pub allow_binary: bool,

    /// Whether to include symbolic links (default: false)
    #[serde(default)]
    pub allow_symlinks: bool,

    /// Root directory override
    pub root: Option<PathBuf>,

    /// Hide this step from the builtins list
    #[serde(default)]
    pub hide: bool,

    /// Test definitions for this step
    #[serde(default)]
    pub tests: IndexMap<String, StepTest>,

    /// How to capture and display output (stderr, stdout, combined, hide)
    #[serde(default)]
    pub output_summary: OutputSummary,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// The mode in which a step runs.
///
/// * `Check` - Verify code without making changes (exit non-zero if issues found)
/// * `Fix` - Automatically fix issues where possible
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    /// Check mode - verify without modifying
    Check,
    /// Fix mode - automatically correct issues
    Fix,
}

/// How command output should be captured for the end-of-run summary.
///
/// This controls what output is shown to the user after all steps complete.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputSummary {
    /// Capture stderr output (default)
    #[default]
    Stderr,
    /// Capture stdout output
    Stdout,
    /// Capture both stdout and stderr combined
    Combined,
    /// Don't capture any output
    Hide,
}

/// A platform-specific script that can vary by operating system.
///
/// Allows defining different commands for different platforms while falling
/// back to a common `other` command when no platform-specific version exists.
///
/// # Example
///
/// ```pkl
/// check {
///     macos = "gfind . -name '*.bak'"
///     linux = "find . -name '*.bak'"
///     other = "find . -name '*.bak'"
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde_as]
pub struct Script {
    /// Command for Linux
    pub linux: Option<String>,
    /// Command for macOS
    pub macos: Option<String>,
    /// Command for Windows
    pub windows: Option<String>,
    /// Fallback command for other platforms (or the default)
    pub other: Option<String>,
}

impl FromStr for Script {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            linux: None,
            macos: None,
            windows: None,
            other: Some(s.to_string()),
        })
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let other = self.other.as_deref().unwrap_or_default();
        if cfg!(target_os = "macos") {
            write!(f, "{}", self.macos.as_deref().unwrap_or(other))
        } else if cfg!(target_os = "linux") {
            write!(f, "{}", self.linux.as_deref().unwrap_or(other))
        } else if cfg!(target_os = "windows") {
            write!(f, "{}", self.windows.as_deref().unwrap_or(other))
        } else {
            write!(f, "{other}")
        }
    }
}
