use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct StepTest {
    /// One of: "check" or "fix"
    #[serde(default)]
    pub run: RunKind,
    /// Files to pass into the template context ({{ files }})
    /// If omitted, defaults to keys from `write`
    pub files: Option<Vec<String>>,
    /// Optional path to copy into a temporary sandbox before running
    pub fixture: Option<String>,
    /// Inline files to create in the sandbox before running
    #[serde(default)]
    pub write: IndexMap<String, String>,
    /// Additional environment just for this test
    #[serde(default)]
    pub env: IndexMap<String, String>,
    /// Expected result of running the test
    #[serde(default)]
    pub expect: StepTestExpect,
    /// Command to run before executing the test command
    pub before: Option<String>,
    /// Command to run after the main command, before evaluating expectations
    pub after: Option<String>,
    /// Whether to run in a temporary directory
    /// If true, always use a sandbox; if false, always use repo root
    /// If None, auto-detect based on whether files reference {{tmp}}
    pub tmpdir: Option<bool>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    #[default]
    Check,
    Fix,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct StepTestExpect {
    #[serde(default)]
    pub code: i32,
    /// Substring which must appear in stdout
    pub stdout: Option<String>,
    /// Substring which must appear in stderr
    pub stderr: Option<String>,
    /// Map of path -> full expected file contents (exact match)
    #[serde(default)]
    pub files: IndexMap<String, String>,
}
