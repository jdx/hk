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
pub struct Validate {
    /// Print the result as a JSON document on stdout
    #[usage(long)]
    pub(crate) json: bool,
    /// Exit with a non-zero status when the config has warnings
    #[usage(long)]
    deny_warnings: bool,
}

impl Validate {
    pub async fn run(&self) -> Result<()> {
        let mut messages: Vec<String> = vec![];
        // Look at the raw evaluation first: loading drops unknown properties
        // in release builds and rejects them in debug builds.
        if let Some(raw) = Config::project_config_json()? {
            for warning in crate::lint::unknown_properties(&raw) {
                self.report(&mut messages, warning);
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
            self.report(&mut messages, warning);
        }
        let path = xx::file::display_path(&config.path);
        if self.json {
            let out = serde_json::json!({
                "path": path,
                "valid": true,
                "warnings": messages,
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
            return self.check_deny_warnings(messages.len());
        }
        let warnings = messages.len();
        if warnings == 0 {
            info!("{path} is valid");
        } else {
            info!("{path} is valid, with {warnings} warning(s)");
        }
        self.check_deny_warnings(warnings)
    }

    /// Log a warning right away so a later error does not hide it. JSON mode
    /// keeps stdout and stderr clean and reports it in the document instead.
    fn report(&self, messages: &mut Vec<String>, warning: String) {
        if !self.json {
            warn!("{warning}");
        }
        messages.push(warning);
    }

    fn check_deny_warnings(&self, warnings: usize) -> Result<()> {
        if self.deny_warnings && warnings > 0 {
            bail!("config has {warnings} warning(s) and --deny-warnings is set");
        }
        Ok(())
    }
}
