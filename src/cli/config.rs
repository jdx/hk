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
        let settings = Settings::get();

        let output = json!({
            "jobs": settings.jobs(),
            "enabled_profiles": settings.enabled_profiles(),
            "disabled_profiles": settings.disabled_profiles(),
            "fail_fast": settings.fail_fast(),
            "display_skip_reasons": settings.display_skip_reasons(),
            "warnings": settings.warnings(),
            "exclude": settings.exclude(),
            "skip_steps": settings.skip_steps(),
            "skip_hooks": settings.skip_hooks(),
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
            "jobs" => json!(settings.jobs()),
            "enabled_profiles" => json!(settings.enabled_profiles()),
            "disabled_profiles" => json!(settings.disabled_profiles()),
            "fail_fast" => json!(settings.fail_fast()),
            "display_skip_reasons" => json!(settings.display_skip_reasons()),
            "warnings" => json!(settings.warnings()),
            "exclude" => json!(settings.exclude()),
            "skip_steps" => json!(settings.skip_steps()),
            "skip_hooks" => json!(settings.skip_hooks()),
            _ => return Err(eyre::eyre!("Unknown configuration key: {}", self.key)),
        };

        println!("{}", serde_json::to_string(&value)?);
        Ok(())
    }
}

impl ConfigExplain {
    fn run(&self) -> Result<()> {
        use crate::settings::SettingsBuilder;

        // Get the current value
        let settings = Settings::get();
        let inner = settings.inner();

        let current_value = match self.key.as_str() {
            // Use existing accessor methods where available
            "jobs" => json!(settings.jobs()),
            "enabled_profiles" => json!(settings.enabled_profiles()),
            "disabled_profiles" => json!(settings.disabled_profiles()),
            "fail_fast" => json!(settings.fail_fast()),
            "display_skip_reasons" => json!(settings.display_skip_reasons()),
            "warnings" => json!(settings.warnings()),
            "exclude" => json!(settings.exclude()),
            "skip_steps" => json!(settings.skip_steps()),
            "skip_hooks" => json!(settings.skip_hooks()),
            "all" => json!(settings.all()),

            // Access inner struct fields directly for everything else
            "quiet" => json!(inner.quiet),
            "silent" => json!(inner.silent),
            "no_progress" => json!(inner.no_progress),
            "slow" => json!(inner.slow),
            "json" => json!(inner.json),
            "libgit2" => json!(inner.libgit2),
            "fix" => json!(inner.fix),
            "check" => json!(inner.check),
            "mise" => json!(inner.mise),
            "stash" => json!(inner.stash),
            "stash_untracked" => json!(inner.stash_untracked),
            "hide_when_done" => json!(inner.hide_when_done),
            "summary_text" => json!(inner.summary_text),
            "check_first" => json!(inner.check_first),
            "log_level" => json!(inner.log_level),
            "log_file_level" => json!(inner.log_file_level),
            "trace" => json!(inner.trace),
            "verbose" => json!(inner.verbose),
            "cache_dir" => json!(inner.cache_dir.as_ref().map(|p| p.display().to_string())),
            "hkrc" => json!(inner.hkrc.display().to_string()),
            "log_file" => json!(inner.log_file.as_ref().map(|p| p.display().to_string())),
            "state_dir" => json!(inner.state_dir.as_ref().map(|p| p.display().to_string())),
            "timing_json" => json!(inner.timing_json.as_ref().map(|p| p.display().to_string())),
            "profiles" => json!(inner.profiles),

            _ => return Err(eyre::eyre!("Unknown configuration key: {}", self.key)),
        };

        // Build a resolution report
        let resolution_info = SettingsBuilder::explain_value(&self.key)?;

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
        println!("4. User rc (.hkrc.pkl)");
        println!("5. Git config (global/system)");
        println!("6. Project config (hk.pkl)");
        println!("7. Built-in defaults");
        println!();
        println!("Note: Use 'hk config dump' to see current effective values");
        Ok(())
    }
}
