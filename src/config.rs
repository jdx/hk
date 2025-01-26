use serde_with::{serde_as, OneOrMany};

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
            "pkl" => {
                rpkl::from_config(path)?
            }
            _ => {
                bail!("Unsupported file extension: {}", ext);
            }
        };
        for hook_group in &mut config.pre_commit {
            for (name, hook) in hook_group {
                hook.name = name.clone();
            }
        }
        Ok(config)
    }
}


#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub pre_commit: Vec<IndexMap<String, Hook>>,
}
