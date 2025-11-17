use crate::version as version_lib;
use std::num::NonZero;
use std::path::PathBuf;
use std::sync::Arc;

use crate::{Result, env, logger, settings::Settings};
use clap::Parser;
use clx::progress::ProgressOutput;

mod builtins;
mod cache;
mod check;
mod completion;
mod config;
mod fix;
mod init;
mod install;
mod migrate;
mod run;
mod test;
mod uninstall;
mod usage;
mod util;
mod validate;
mod version;

#[derive(clap::Parser)]
#[clap(name = "hk", version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION"), version = version_lib::version())]
struct Cli {
    /// Path to user configuration file
    #[clap(long, global = true, value_name = "PATH")]
    hkrc: Option<PathBuf>,
    /// Number of jobs to run in parallel
    #[clap(short, long, global = true)]
    jobs: Option<NonZero<usize>>,
    /// Profiles to enable/disable
    /// prefix with ! to disable
    /// e.g. --profile slow --profile !fast
    #[clap(short, long, global = true)]
    profile: Vec<String>,
    /// Shorthand for --profile=slow
    #[clap(short, long, global = true)]
    slow: bool,
    /// Enables verbose output
    #[clap(short, long, global = true, action = clap::ArgAction::Count, overrides_with_all = ["quiet", "silent"])]
    verbose: u8,
    /// Disables progress output
    #[clap(short, long, global = true)]
    no_progress: bool,
    /// Suppresses output
    #[clap(short, long, global = true, overrides_with_all = ["verbose", "silent"])]
    quiet: bool,
    /// Suppresses all output
    #[clap(long, global = true, overrides_with_all = ["quiet", "verbose"])]
    silent: bool,
    /// Enable tracing spans and performance diagnostics
    #[clap(long, global = true)]
    trace: bool,
    /// Output in JSON format
    #[clap(long, global = true)]
    json: bool,
    #[clap(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Builtins(Box<builtins::Builtins>),
    Cache(Box<cache::Cache>),
    Check(Box<check::Check>),
    Completion(Box<completion::Completion>),
    Config(Box<config::Config>),
    Fix(Box<fix::Fix>),
    Init(Box<init::Init>),
    Install(Box<install::Install>),
    Migrate(Box<migrate::Migrate>),
    Run(Box<run::Run>),
    Test(Box<test::Test>),
    Uninstall(Box<uninstall::Uninstall>),
    Usage(Box<usage::Usage>),
    Util(Box<util::Util>),
    Validate(Box<validate::Validate>),
    Version(Box<version::Version>),
}

pub async fn run() -> Result<()> {
    let args = Cli::parse();

    // Determine effective log level from CLI flags (env default applied by logger if None)
    let mut level: Option<log::LevelFilter> = None;
    // Derive verbosity overrides first
    let config_path = if let Some(custom_path) = args.hkrc {
        custom_path
    } else {
        PathBuf::from(".hkrc.pkl")
    };
    Settings::set_cli_snapshot(crate::settings::CliSnapshot {
        hkrc: Some(config_path),
        jobs: args.jobs.map(|n| n.get()),
        profiles: args.profile.clone(),
        slow: args.slow,
        quiet: args.quiet,
        silent: args.silent,
    });

    if !console::user_attended_stderr() || args.no_progress {
        clx::progress::set_output(ProgressOutput::Text);
    }
    if args.verbose > 1 {
        clx::progress::set_output(ProgressOutput::Text);
        level = Some(log::LevelFilter::Trace);
    }
    if args.verbose == 1 {
        clx::progress::set_output(ProgressOutput::Text);
        level = Some(log::LevelFilter::Debug);
    }
    if args.quiet {
        clx::progress::set_output(ProgressOutput::Text);
        level = Some(log::LevelFilter::Warn);
    }
    if args.silent {
        clx::progress::set_output(ProgressOutput::Text);
        level = Some(log::LevelFilter::Error);
    }

    // Decide tracing enablement and output format
    // Support: --trace, HK_TRACE mode (Text/Json), or effective log level TRACE
    let json_output = args.json || *env::HK_JSON || matches!(*env::HK_TRACE, env::TraceMode::Json);

    let mut trace_enabled =
        args.trace || matches!(*env::HK_TRACE, env::TraceMode::Text | env::TraceMode::Json);

    let effective_level = level.unwrap_or(*env::HK_LOG);
    if effective_level == log::LevelFilter::Trace {
        trace_enabled = true;
    }

    // Set text progress output for debug/trace levels to prevent interference
    if effective_level == log::LevelFilter::Debug || effective_level == log::LevelFilter::Trace {
        clx::progress::set_output(ProgressOutput::Text);
    }

    // Initialize logger first so regular log records are handled by our logger (and not forwarded to tracing)
    logger::init(level);
    if trace_enabled {
        clx::progress::set_output(ProgressOutput::Text);
        crate::trace::init_tracing(json_output)?;
    }

    // Only load settings if not running migrate command to avoid config loading errors
    // during migration with potentially invalid existing configs
    let settings = if matches!(args.command, Commands::Migrate(_)) {
        // For migrate, use minimal default settings to avoid loading invalid configs
        Arc::new(crate::settings::generated::settings::Settings::default())
    } else {
        Settings::get()
    };
    if !settings.terminal_progress {
        clx::osc::configure(settings.terminal_progress);
    }

    // CLI settings snapshot applied above; settings are built from snapshot
    match args.command {
        Commands::Builtins(cmd) => cmd.run().await,
        Commands::Cache(cmd) => cmd.run().await,
        Commands::Check(cmd) => cmd.hook.run("check").await,
        Commands::Completion(cmd) => cmd.run().await,
        Commands::Config(cmd) => cmd.run().await,
        Commands::Fix(cmd) => cmd.hook.run("fix").await,
        Commands::Init(cmd) => cmd.run().await,
        Commands::Install(cmd) => cmd.run().await,
        Commands::Migrate(cmd) => cmd.run().await,
        Commands::Run(cmd) => cmd.run().await,
        Commands::Uninstall(cmd) => cmd.run().await,
        Commands::Usage(cmd) => cmd.run().await,
        Commands::Util(cmd) => cmd.run().await,
        Commands::Validate(cmd) => cmd.run().await,
        Commands::Version(cmd) => cmd.run().await,
        Commands::Test(cmd) => cmd.run().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_subcommands_are_sorted() {
        let cmd = Cli::command();
        // Check all subcommands for alphabetical ordering
        for subcmd in cmd.get_subcommands() {
            clap_sort::assert_sorted(subcmd);
        }
    }
}
