use eyre::bail;

use crate::{Result, config::Config};

/// Validate the config file
///
/// Errors for configs hk cannot run. Also prints warnings for settings that
/// load but probably do not do what was meant: unknown properties (which
/// release builds silently drop), `depends` that never orders anything,
/// steps without a command, and globs that cannot match.
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Validate;

impl Validate {
    pub async fn run(&self) -> Result<()> {
        let mut warnings = 0;
        // Look at the raw evaluation first: loading drops unknown properties
        // in release builds and rejects them in debug builds.
        if let Some(raw) = Config::project_config_json()? {
            for warning in crate::lint::unknown_properties(&raw) {
                warn!("{warning}");
                warnings += 1;
            }
        }
        let config = Config::get()?;
        config.validate()?;
        if !config.path.exists() {
            bail!(
                "config file {} does not exist",
                xx::file::display_path(&config.path)
            );
        }
        for warning in config.lint() {
            warn!("{warning}");
            warnings += 1;
        }
        let path = xx::file::display_path(&config.path);
        if warnings == 0 {
            info!("{path} is valid");
        } else {
            info!("{path} is valid, with {warnings} warning(s)");
        }
        Ok(())
    }
}
