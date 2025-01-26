use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{hook::Hook, Result};
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub pre_commit: Vec<Hook>,
}

impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        let raw = xx::file::read_to_string(path)?;
        let config: Config = toml::from_str(&raw)?;
        Ok(config)
    }
}
