
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{bail, hook::Hook, Result};

impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        let ext = path.extension().unwrap_or_default().to_str().unwrap();
        let mut config : Config = match ext {
            "toml" => {
                let raw = xx::file::read_to_string(path)?;
                toml::from_str(&raw)?
            }
            "yaml" => {
                let raw = xx::file::read_to_string(path)?;
                serde_yaml::from_str(&raw)?
            }
            "json" => {
                let raw = xx::file::read_to_string(path)?;
                serde_json::from_str(&raw)?
            }
            "pkl" => {
                rpkl::from_config(path)?
            }
            _ => {
                bail!("Unsupported file extension: {}", ext);
            }
        };
        for (name, hook) in &mut config.pre_commit {
            hook.name = name.clone();
        }
        Ok(config)
    }
}


#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub pre_commit: IndexMap<String, Hook>,
    pub pre_push: IndexMap<String, Hook>,
}
