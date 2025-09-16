use crate::{Result, config::Config as HKConfig, settings::Settings};
use serde_json::json;

/// Configuration introspection and management
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "cfg")]
pub struct Config {
    #[clap(subcommand)]
    command: Option<ConfigCommand>,
}

#[derive(Debug, clap::Subcommand)]
enum ConfigCommand {
    /// Print effective runtime settings (JSON format)
    Dump(ConfigDump),
    /// Get a specific configuration value
    Get(ConfigGet),
    /// Show the source of each configuration value
    Sources(ConfigSources),
    /// Show the configuration file (deprecated - use without subcommand instead)
    Show,
}

#[derive(Debug, clap::Args)]
struct ConfigDump {
    /// Output format
    #[clap(long, value_parser = ["json", "toml"], default_value = "json")]
    format: String,
}

#[derive(Debug, clap::Args)]
struct ConfigGet {
    /// Configuration key to retrieve
    key: String,
}

#[derive(Debug, clap::Args)]
struct ConfigSources {}

impl Config {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            None | Some(ConfigCommand::Show) => {
                warn!("this output is almost certain to change in a future version");
                let cfg = HKConfig::get()?;
                println!("{cfg}");
            }
            Some(ConfigCommand::Dump(cmd)) => cmd.run()?,
            Some(ConfigCommand::Get(cmd)) => cmd.run()?,
            Some(ConfigCommand::Sources(cmd)) => cmd.run()?,
        }
        Ok(())
    }
}

impl ConfigDump {
    fn run(&self) -> Result<()> {
        let settings = Settings::get();

        let output = json!({
            "jobs": settings.jobs,
            "enabled_profiles": settings.enabled_profiles,
            "disabled_profiles": settings.disabled_profiles,
            "fail_fast": settings.fail_fast,
            "display_skip_reasons": settings.display_skip_reasons,
            "warnings": settings.warnings,
            "exclude_paths": settings.exclude_paths,
            "exclude_globs": settings.exclude_globs,
        });

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
        let settings = Settings::get();

        let value = match self.key.as_str() {
            "jobs" => json!(settings.jobs),
            "enabled_profiles" => json!(settings.enabled_profiles),
            "disabled_profiles" => json!(settings.disabled_profiles),
            "fail_fast" => json!(settings.fail_fast),
            "display_skip_reasons" => json!(settings.display_skip_reasons),
            "warnings" => json!(settings.warnings),
            "exclude_paths" => json!(settings.exclude_paths),
            "exclude_globs" => json!(settings.exclude_globs),
            _ => return Err(eyre::eyre!("Unknown configuration key: {}", self.key)),
        };

        println!("{}", serde_json::to_string(&value)?);
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
        println!("4. User rc (.hkrc.pkl)");
        println!("5. Git config (global/system)");
        println!("6. Project config (hk.pkl)");
        println!("7. Built-in defaults");
        println!();
        println!("Note: Use 'hk config dump' to see current effective values");
        Ok(())
    }
}
