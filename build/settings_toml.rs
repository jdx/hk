use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SettingsRegistry {
    #[serde(flatten)]
    pub options: IndexMap<String, OptionConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OptionConfig {
    #[serde(rename = "type")]
    pub typ: String,
    pub default: Option<toml::Value>,
    #[serde(default)]
    pub merge: Option<String>,
    pub sources: SourcesConfig,
    #[serde(default)]
    pub validate: Option<ValidateConfig>,
    pub docs: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub deprecated: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourcesConfig {
    #[serde(default)]
    pub cli: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub git: Vec<String>,
    #[serde(default)]
    pub pkl: PklSource,
}

#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
pub enum PklSource {
    #[default]
    None,
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ValidateConfig {
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}
