use eyre::bail;

use crate::{Result, config::Config};

/// Validate the config file
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Validate;

impl Validate {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        config.validate()?;
        if !config.path.exists() {
            bail!(
                "config file {} does not exist",
                xx::file::display_path(&config.path)
            );
        }
        info!("{} is valid", xx::file::display_path(&config.path));
        Ok(())
    }
}
