use crate::version as version_lib;
use std::num::NonZero;
use std::path::PathBuf;

use crate::{Result, logger, settings::Settings};
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
mod run;
mod uninstall;
mod usage;
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
    Run(Box<run::Run>),
    Usage(Box<usage::Usage>),
    Uninstall(Box<uninstall::Uninstall>),
    Validate(Box<validate::Validate>),
    Version(Box<version::Version>),
}

pub async fn run() -> Result<()> {
    let args = Cli::parse();
    let mut level = None;
    let config_path = if let Some(custom_path) = args.hkrc {
        custom_path
    } else {
        PathBuf::from(".hkrc.pkl")
    };
    Settings::set_user_config_path(config_path);

    if !console::user_attended_stderr() || args.no_progress {
        clx::progress::set_output(ProgressOutput::Text);
    }
    if args.verbose > 1 || log::log_enabled!(log::Level::Trace) {
        clx::progress::set_output(ProgressOutput::Text);
        level = Some(log::LevelFilter::Trace);
    }
    if args.verbose == 1 || log::log_enabled!(log::Level::Debug) {
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
    logger::init(level);
    if let Some(jobs) = args.jobs {
        Settings::set_jobs(jobs);
    }
    if !args.profile.is_empty() {
        Settings::with_profiles(&args.profile);
    }
    if args.slow {
        Settings::with_profiles(&["slow".to_string()]);
    }
    match args.command {
        Commands::Builtins(cmd) => cmd.run().await,
        Commands::Cache(cmd) => cmd.run().await,
        Commands::Check(cmd) => cmd.hook.run("check").await,
        Commands::Completion(cmd) => cmd.run().await,
        Commands::Config(cmd) => cmd.run().await,
        Commands::Fix(cmd) => cmd.hook.run("fix").await,
        Commands::Init(cmd) => cmd.run().await,
        Commands::Install(cmd) => cmd.run().await,
        Commands::Run(cmd) => cmd.run().await,
        Commands::Uninstall(cmd) => cmd.run().await,
        Commands::Usage(cmd) => cmd.run().await,
        Commands::Validate(cmd) => cmd.run().await,
        Commands::Version(cmd) => cmd.run().await,
    }
}
