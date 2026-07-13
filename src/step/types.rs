//! Core type definitions for step configuration.
//!
//! This module contains the fundamental types used to define and configure steps:
//! - [`Step`] - The main configuration struct for a linting/formatting step
//! - [`FileSelector`] - A positive file selector used by `match_any`
//! - [`Pattern`] - File matching patterns (globs or regex)
//! - [`Command`] - Shell scripts or structured argument vectors
//! - [`Script`] - Platform-specific shell scripts
//! - [`RunType`] - Whether to run in check or fix mode
//! - [`OutputSummary`] - How to capture and display command output

use crate::{Result, step_test::StepTest, tera};
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
        _type: String,
        /// The regex pattern string
        pattern: String,
    },
    /// One or more glob patterns
    Globs(Vec<String>),
}

impl Pattern {
    pub fn is_empty(&self) -> bool {
        match self {
            Pattern::Regex { .. } => false,
            Pattern::Globs(globs) => globs.is_empty(),
        }
    }
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

/// A positive file-selection clause.
///
/// Glob and type filters within one selector use AND semantics. Multiple
/// selectors in [`Step::match_any`] use OR semantics.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct FileSelector {
    /// File matching pattern (globs or regex)
    #[serde(default)]
    pub glob: Option<Pattern>,

    /// File types to match
    #[serde(default)]
    pub types: Option<Vec<String>>,
}

impl FileSelector {
    pub fn is_empty(&self) -> bool {
        self.glob.as_ref().is_none_or(Pattern::is_empty)
            && self.types.as_ref().is_none_or(Vec::is_empty)
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

    /// Alternative positive file selectors, combined with OR semantics
    #[serde(default)]
    pub match_any: Option<Vec<FileSelector>>,

    /// Whether this step requires interactive terminal input
    #[serde(default)]
    pub interactive: bool,

    /// Content to pipe to the command's stdin
    pub stdin: Option<String>,

    /// Environment variables that must be set for this step to run
    #[serde(default)]
    pub required: Vec<String>,

    /// Steps that must complete before this one runs
    #[serde(default)]
    pub depends: Vec<String>,

    /// Custom shell to use (default: `sh -o errexit -c`)
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub shell: Option<Script>,

    /// Command to check for issues (exit non-zero if issues found)
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check: Option<Command>,

    /// Command that outputs a list of files needing fixes (one per line)
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check_list_files: Option<Command>,

    /// Command that outputs a unified diff of needed changes
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check_diff: Option<Command>,

    /// Command to fix issues
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub fix: Option<Command>,

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
    #[serde(default)]
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

impl RunType {
    pub fn as_str(self) -> &'static str {
        match self {
            RunType::Check => "check",
            RunType::Fix => "fix",
        }
    }
}

impl fmt::Display for RunType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
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

/// A command represented as an executable followed by its arguments.
///
/// Exact `{{files}}` and `{{workspace_files}}` entries expand to multiple
/// arguments. Every other entry is rendered as one Tera-templated argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArgvCommand {
    pub argv: Vec<String>,
}

/// A step command that either runs through a shell or executes an argv directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Command {
    Argv(ArgvCommand),
    Shell(Script),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RenderedCommand {
    Shell(String),
    Argv(Vec<String>),
}

impl Command {
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Shell(script) => script.to_string().trim().is_empty(),
            Self::Argv(command) => command.argv.is_empty(),
        }
    }

    pub fn is_argv(&self) -> bool {
        matches!(self, Self::Argv(_))
    }

    pub(crate) fn render(
        &self,
        tctx: &tera::Context,
        prefix: Option<&str>,
    ) -> Result<RenderedCommand> {
        match self {
            Self::Shell(script) => {
                let script = script.to_string();
                let script = if let Some(prefix) = prefix {
                    format!("{prefix} {script}")
                } else {
                    script
                };
                Ok(RenderedCommand::Shell(tera::render(&script, tctx)?))
            }
            Self::Argv(command) => {
                if prefix.is_some() {
                    eyre::bail!("structured argv commands cannot use `prefix`");
                }
                let mut rendered = Vec::new();
                for (index, arg) in command.argv.iter().enumerate() {
                    let placeholder = arg
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>();
                    let list = match placeholder.as_str() {
                        "{{files}}" => Some("files_list"),
                        "{{workspace_files}}" => Some("workspace_files_list"),
                        _ => None,
                    };
                    if let Some(list) = list {
                        if index == 0 {
                            eyre::bail!("the executable cannot be a file-list placeholder");
                        }
                        rendered.extend(tctx.string_list(list).unwrap_or_default());
                    } else {
                        rendered.push(tera::render(arg, tctx)?);
                    }
                }
                if rendered.is_empty() || rendered[0].trim().is_empty() {
                    eyre::bail!("structured argv command must contain an executable");
                }
                Ok(RenderedCommand::Argv(rendered))
            }
        }
    }
}

impl RenderedCommand {
    pub(crate) fn execution_size(&self) -> usize {
        match self {
            Self::Shell(script) => script.len(),
            Self::Argv(argv) => argv.iter().map(|arg| arg.len() + 1).sum(),
        }
    }

    pub(crate) fn display(&self, shell_type: super::ShellType) -> String {
        match self {
            Self::Shell(script) => script.clone(),
            Self::Argv(argv) => argv
                .iter()
                .map(|arg| shell_type.quote(arg))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

impl FromStr for Command {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::Shell(s.parse()?))
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shell(script) => script.fmt(f),
            Self::Argv(command) => write!(f, "{}", command.argv.join(" ")),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckFirstCmd<'a> {
    Diff(&'a Command),
    ListFiles(&'a Command),
    Check(&'a Command),
}

impl<'a> CheckFirstCmd<'a> {
    pub(crate) fn command(self) -> &'a Command {
        match self {
            Self::Diff(command) | Self::ListFiles(command) | Self::Check(command) => command,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_argv_expands_file_lists_as_distinct_arguments() {
        let command = Command::Argv(ArgvCommand {
            argv: vec![
                "tool".to_string(),
                "--label={{color}}".to_string(),
                "{{files}}".to_string(),
                "{{workspace_files}}".to_string(),
            ],
        });
        let mut tctx = tera::Context::default();
        tctx.insert("color", "blue");
        tctx.insert("files_list", &vec!["a b.txt", "semi;colon.txt"]);
        tctx.insert("workspace_files_list", &vec!["src/lib.rs"]);

        assert_eq!(
            command.render(&tctx, None).unwrap(),
            RenderedCommand::Argv(vec![
                "tool".to_string(),
                "--label=blue".to_string(),
                "a b.txt".to_string(),
                "semi;colon.txt".to_string(),
                "src/lib.rs".to_string(),
            ])
        );
    }

    #[test]
    fn structured_argv_rejects_shell_prefix() {
        let command = Command::Argv(ArgvCommand {
            argv: vec!["tool".to_string()],
        });

        let err = command
            .render(&tera::Context::default(), Some("env FOO=bar"))
            .unwrap_err();

        assert!(err.to_string().contains("cannot use `prefix`"));
    }

    #[test]
    fn test_script_empty_windows_command() {
        // Test that an empty windows command results in an empty string on Windows
        let script = Script {
            linux: Some("linux_cmd".to_string()),
            macos: Some("macos_cmd".to_string()),
            windows: Some("".to_string()),
            other: None,
        };

        #[cfg(target_os = "windows")]
        {
            assert_eq!(script.to_string(), "");
            assert!(script.to_string().trim().is_empty());
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On non-Windows, should use platform-specific command
            assert!(!script.to_string().is_empty());
        }
    }

    #[test]
    fn test_script_none_windows_command_with_other() {
        // Test that None windows with Some other falls back to other
        let script = Script {
            linux: None,
            macos: None,
            windows: None,
            other: Some("fallback_cmd".to_string()),
        };

        // On all platforms, should use the fallback
        assert_eq!(script.to_string(), "fallback_cmd");
    }

    #[test]
    fn test_script_all_none_produces_empty() {
        // Test that all None produces empty string
        let script = Script {
            linux: None,
            macos: None,
            windows: None,
            other: None,
        };

        assert_eq!(script.to_string(), "");
        assert!(script.to_string().trim().is_empty());
    }
}
