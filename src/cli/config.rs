use crate::settings::generated::SETTINGS_META;
use crate::{Result, settings::Settings};
use serde_json::json;

/// Configuration introspection and management
///
/// View and inspect hk's configuration from all sources.
/// Configuration is merged from multiple sources in precedence order:
/// CLI flags > Environment variables > Git config (local) > User config (.hkrc.pkl) >
/// Git config (global) > Project config (hk.pkl) > Built-in defaults.
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "cfg")]
pub struct Config {
    #[clap(subcommand)]
    command: Option<ConfigCommand>,
}

#[derive(Debug, clap::Subcommand)]
enum ConfigCommand {
    /// Print effective runtime settings (JSON format)
    ///
    /// Shows the merged configuration from all sources including CLI flags,
    /// environment variables, git config, user config, and project config.
    Dump(ConfigDump),
    /// Explain where a configuration value comes from
    ///
    /// Shows the resolved value, its source (env/git/cli/default), and
    /// the full precedence chain showing all layers that could affect it.
    Explain(ConfigExplain),
    /// Get a specific configuration value
    ///
    /// Available keys: jobs, enabled_profiles, disabled_profiles, fail_fast,
    /// display_skip_reasons, warnings, exclude, skip_steps, skip_hooks
    Get(ConfigGet),
    /// Show the configuration source precedence order
    ///
    /// Lists all configuration sources in order of precedence to help
    /// understand where configuration values come from.
    Sources(ConfigSources),
}

#[derive(Debug, clap::Args)]
struct ConfigDump {
    /// Output format (json or toml)
    #[clap(long, value_parser = ["json", "toml"], default_value = "json")]
    format: String,
}

#[derive(Debug, clap::Args)]
struct ConfigGet {
    /// Configuration key to retrieve
    ///
    /// Available keys: jobs, enabled_profiles, disabled_profiles, fail_fast,
    /// display_skip_reasons, warnings, exclude, skip_steps, skip_hooks
    key: String,
}

#[derive(Debug, clap::Args)]
struct ConfigExplain {
    /// Configuration key to explain
    key: String,
}

#[derive(Debug, clap::Args)]
struct ConfigSources {}

impl Config {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Some(ConfigCommand::Dump(cmd)) => cmd.run(),
            Some(ConfigCommand::Get(cmd)) => cmd.run(),
            Some(ConfigCommand::Explain(cmd)) => cmd.run(),
            Some(ConfigCommand::Sources(cmd)) => cmd.run(),
            None => {
                warn!("this output is almost certain to change in a future version");
                let dump = ConfigDump {
                    format: "toml".to_string(),
                };
                dump.run()
            }
        }
    }
}

impl ConfigDump {
    fn run(&self) -> Result<()> {
        let settings = Settings::try_get()?;
        // Start from full settings based on meta to reduce boilerplate
        let mut map = serde_json::Map::new();
        // Serialize full settings once for generic lookups
        let full = serde_json::to_value(settings.clone())?;
        for (key, _meta) in SETTINGS_META.iter() {
            let k = (*key).to_string();
            // Special-case computed values that differ from raw fields
            if k == "jobs" {
                map.insert(k, json!(settings.jobs()));
                continue;
            }
            if let Some(v) = full.get(key) {
                map.insert(k, v.clone());
            }
        }
        // Include derived convenience fields expected by CLI/tests
        map.insert(
            "enabled_profiles".to_string(),
            json!(settings.enabled_profiles()),
        );
        map.insert(
            "disabled_profiles".to_string(),
            json!(settings.disabled_profiles()),
        );
        let output = serde_json::Value::Object(map);

        match self.format.as_str() {
            "json" => println!("{}", serde_json::to_string_pretty(&output)?),
            "toml" => {
                let toml_value: toml::Value = serde_json::from_value(output)?;
                println!("{}", toml::to_string_pretty(&toml_value)?);
            }
            _ => unreachable!("Invalid format"),
        }
        Ok(())
    }
}

impl ConfigGet {
    fn run(&self) -> Result<()> {
        let settings = Settings::try_get()?;
        // Derived and computed keys
        let value = if self.key == "jobs" {
            json!(settings.jobs())
        } else if self.key == "enabled_profiles" {
            json!(settings.enabled_profiles())
        } else if self.key == "disabled_profiles" {
            json!(settings.disabled_profiles())
        } else if SETTINGS_META.contains_key(self.key.as_str()) {
            // Generic lookup via serialization
            let full = serde_json::to_value(settings.clone())?;
            full.get(&self.key).cloned().ok_or_else(|| {
                eyre::eyre!("Key present in meta but missing in settings: {}", self.key)
            })?
        } else {
            return Err(eyre::eyre!("Unknown configuration key: {}", self.key));
        };

        println!("{}", serde_json::to_string(&value)?);
        Ok(())
    }
}

impl ConfigExplain {
    fn run(&self) -> Result<()> {
        // Get the current value
        let settings = Settings::try_get()?;
        // Current value (computed for special keys, generic via meta for the rest)
        let current_value = if self.key == "jobs" {
            json!(settings.jobs())
        } else if self.key == "enabled_profiles" {
            json!(settings.enabled_profiles())
        } else if self.key == "disabled_profiles" {
            json!(settings.disabled_profiles())
        } else if SETTINGS_META.contains_key(self.key.as_str()) {
            let full = serde_json::to_value(settings.clone())?;
            full.get(&self.key).cloned().ok_or_else(|| {
                eyre::eyre!("Key present in meta but missing in settings: {}", self.key)
            })?
        } else {
            return Err(eyre::eyre!("Unknown configuration key: {}", self.key));
        };

        // Build a resolution report
        let resolution_info = Settings::explain_value(&self.key)?;

        println!("Configuration key: {}", self.key);
        println!("Current value: {}", serde_json::to_string(&current_value)?);
        println!();
        println!("{}", resolution_info);

        Ok(())
    }
}

impl ConfigSources {
    fn run(&self) -> Result<()> {
        // For now, we'll just show that the values come from the merged settings
        // In a more complete implementation, we'd track where each value originated
        println!("Configuration sources (in order of precedence):");
        println!("1. CLI flags");
        println!("2. Environment variables (HK_*)");
        println!("3. Git config (local repo)");
        println!("4. Git config (global/system)");
        println!("5. User rc (.hkrc.pkl)");
        println!("6. Project config (hk.pkl)");
        println!("7. Built-in defaults");
        println!();
        println!("Note: Use 'hk config dump' to see current effective values");
        Ok(())
    }
}
