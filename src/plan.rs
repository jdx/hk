use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub hook: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub status: StepStatus,
    #[serde(rename = "orderIndex")]
    pub order_index: usize,
    #[serde(rename = "parallelGroupId", skip_serializing_if = "Option::is_none")]
    pub parallel_group_id: Option<String>,
    #[serde(rename = "dependsOn", skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    pub reasons: Vec<Reason>,
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
    Dependency,
    FilterMatch,
    FilterNoMatch,
    ChangedFilesMatch,
    ChangedFilesNoMatch,
    ProfileInclude,
    ProfileExclude,
    ConditionTrue,
    ConditionFalse,
    ConditionUnknown,
    CliInclude,
    CliExclude,
    Always,
    Disabled,
}

impl ReasonKind {
    pub fn short_description(&self) -> &str {
        match self {
            ReasonKind::Dependency => "required by dependency",
            ReasonKind::FilterMatch => "files matched filters",
            ReasonKind::FilterNoMatch => "no files matched filters",
            ReasonKind::ChangedFilesMatch => "changed files matched",
            ReasonKind::ChangedFilesNoMatch => "no changed files matched",
            ReasonKind::ProfileInclude => "included by profile",
            ReasonKind::ProfileExclude => "excluded by profile",
            ReasonKind::ConditionTrue => "condition evaluated to true",
            ReasonKind::ConditionFalse => "condition evaluated to false",
            ReasonKind::ConditionUnknown => "condition could not be evaluated",
            ReasonKind::CliInclude => "included via CLI",
            ReasonKind::CliExclude => "excluded via CLI",
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
    pub fn new(hook: String) -> Self {
        Self {
            hook,
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
