use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct StepTest {
    /// One of: "check", "fix", or "command"
    #[serde(default = "default_run_kind")]
    pub run: RunKind,
    /// Raw command to run when run == command
    pub command: Option<String>,
    /// Files to pass into the template context ({{ files }})
    #[serde(default)]
    pub files: Vec<String>,
    /// Optional path to copy into a temporary sandbox before running
    pub fixture: Option<String>,
    /// Inline files to create in the sandbox before running
    #[serde(default)]
    pub write: IndexMap<String, String>,
    /// Additional environment just for this test
    #[serde(default)]
    pub env: IndexMap<String, String>,
    #[serde(default)]
    pub expect: StepTestExpect,
}

fn default_run_kind() -> RunKind {
    RunKind::Check
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    Check,
    Fix,
    Command,
}

impl Default for RunKind {
    fn default() -> Self {
        RunKind::Check
    }
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct StepTestExpect {
    #[serde(default)]
    pub code: i32,
    /// Substrings which must appear in stdout
    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default)]
    pub stdout: Vec<String>,
    /// Substrings which must appear in stderr
    #[serde_as(as = "OneOrMany<_>")]
    #[serde(default)]
    pub stderr: Vec<String>,
    /// Map of path -> full expected file contents (exact match)
    #[serde(default)]
    pub files: IndexMap<String, String>,
}
