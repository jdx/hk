use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub hook: String,
    #[serde(rename = "runType")]
    pub run_type: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<String>,
    pub steps: Vec<PlannedStep>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ParallelGroup>,
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    pub name: String,
    pub status: StepStatus,
    #[serde(rename = "orderIndex")]
    pub order_index: usize,
    #[serde(rename = "parallelGroupId", skip_serializing_if = "Option::is_none")]
    pub parallel_group_id: Option<String>,
    #[serde(rename = "dependsOn", skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    pub reasons: Vec<Reason>,
    #[serde(rename = "fileCount", skip_serializing_if = "Option::is_none")]
    pub file_count: Option<usize>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Included,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reason {
    pub kind: ReasonKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReasonKind {
    FilterMatch,
    FilterNoMatch,
    ProfileInclude,
    ProfileExclude,
    ConditionTrue,
    ConditionFalse,
    ConditionUnknown,
    CliInclude,
    CliExclude,
    EnvExclude,
    ConfigExclude,
    NoCommand,
    MissingRequiredEnv,
    Always,
    Disabled,
}

impl ReasonKind {
    /// Returns true for reasons that explain why a step will NOT run.
    ///
    /// Used by the text renderer to pick a headline that actually matches the
    /// step's status — a Skipped step shouldn't show "condition evaluated to
    /// true" as its headline just because the truthy-condition reason was
    /// pushed first.
    pub fn is_skip(&self) -> bool {
        matches!(
            self,
            ReasonKind::FilterNoMatch
                | ReasonKind::ProfileExclude
                | ReasonKind::ConditionFalse
                | ReasonKind::CliExclude
                | ReasonKind::EnvExclude
                | ReasonKind::ConfigExclude
                | ReasonKind::NoCommand
                | ReasonKind::MissingRequiredEnv
                | ReasonKind::Disabled
        )
    }

    pub fn short_description(&self) -> &str {
        match self {
            ReasonKind::FilterMatch => "files matched filters",
            ReasonKind::FilterNoMatch => "no files matched filters",
            ReasonKind::ProfileInclude => "included by profile",
            ReasonKind::ProfileExclude => "excluded by profile",
            ReasonKind::ConditionTrue => "condition evaluated to true",
            ReasonKind::ConditionFalse => "condition evaluated to false",
            ReasonKind::ConditionUnknown => "condition could not be evaluated",
            ReasonKind::CliInclude => "included via CLI",
            ReasonKind::CliExclude => "excluded via CLI",
            ReasonKind::EnvExclude => "excluded via environment",
            ReasonKind::ConfigExclude => "excluded via config",
            ReasonKind::NoCommand => "no command for run type",
            ReasonKind::MissingRequiredEnv => "required environment variable not set",
            ReasonKind::Always => "always runs",
            ReasonKind::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelGroup {
    pub id: String,
    #[serde(rename = "stepIds")]
    pub step_ids: Vec<String>,
}

impl Plan {
    pub fn new(hook: String, run_type: String) -> Self {
        Self {
            hook,
            run_type,
            profiles: Vec::new(),
            steps: Vec::new(),
            groups: Vec::new(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn add_step(&mut self, step: PlannedStep) {
        self.steps.push(step);
    }

    pub fn add_group(&mut self, group: ParallelGroup) {
        self.groups.push(group);
    }

    pub fn with_profiles(mut self, profiles: Vec<String>) -> Self {
        self.profiles = profiles;
        self
    }
}
