use std::path::Path;

use crate::{config::Config as HKConfig, Result};

/// Generate a default hk.toml configuration file
#[derive(Debug, clap::Args)]
#[clap(alias="cfg")]
pub struct Config {}

impl Config {
    pub async fn run(&self) -> Result<()> {
        let cfg = HKConfig::read(Path::new("hk.pkl"))?;
        Ok(())
    }
} 
