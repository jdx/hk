use crate::version as version_lib;
use std::num::NonZero;

use crate::{Result, logger, settings::Settings};
use clap::Parser;
use clx::progress::ProgressOutput;

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
    Cache(cache::Cache),
    Check(check::Check),
    Completion(completion::Completion),
    Config(config::Config),
    Fix(fix::Fix),
    Init(init::Init),
    Install(install::Install),
    Run(run::Run),
    Usage(usage::Usage),
    Uninstall(uninstall::Uninstall),
    Validate(validate::Validate),
    Version(version::Version),
}

pub async fn run() -> Result<()> {
    let args = Cli::parse();
    let mut level = None;
    if !console::user_attended_stderr() {
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
    let settings = Settings::get();
    if let Some(jobs) = args.jobs {
        settings.set_jobs(jobs);
    }
    if !args.profile.is_empty() {
        settings.with_profiles(&args.profile);
    }
    if args.slow {
        settings.with_profiles(&["slow".to_string()]);
    }
    match args.command {
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
