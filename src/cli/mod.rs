use crate::version as version_lib;
use std::num::NonZero;

use crate::{logger, settings::Settings, Result};
use clap::Parser;

mod cache;
mod check;
mod completion;
mod config;
mod fix;
mod generate;
mod install;
mod run;
mod usage;
mod version;

#[derive(Debug, clap::Parser)]
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

#[derive(Debug, clap::Subcommand)]
enum Commands {
    Cache(cache::Cache),
    Check(check::Check),
    Completion(completion::Completion),
    Config(config::Config),
    Fix(fix::Fix),
    Generate(generate::Generate),
    Install(install::Install),
    Run(run::Run),
    Usage(usage::Usage),
    Version(version::Version),
}

pub async fn run() -> Result<()> {
    let args = Cli::parse();
    let mut level = None;
    if args.verbose > 1 || log::log_enabled!(log::Level::Trace) {
        clx::MultiProgressReport::set_output_type(clx::OutputType::Verbose);
        level = Some(log::LevelFilter::Trace);
    }
    if args.verbose == 1 || log::log_enabled!(log::Level::Debug) {
        clx::MultiProgressReport::set_output_type(clx::OutputType::Verbose);
        level = Some(log::LevelFilter::Debug);
    }
    if args.quiet {
        clx::MultiProgressReport::set_output_type(clx::OutputType::Quiet);
        level = Some(log::LevelFilter::Warn);
    }
    if args.silent {
        clx::MultiProgressReport::set_output_type(clx::OutputType::Quiet);
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
    match args.command {
        Commands::Cache(cmd) => cmd.run().await,
        Commands::Check(cmd) => cmd.run().await,
        Commands::Completion(cmd) => cmd.run().await,
        Commands::Config(cmd) => cmd.run().await,
        Commands::Fix(cmd) => cmd.run().await,
        Commands::Generate(cmd) => cmd.run().await,
        Commands::Install(cmd) => cmd.run().await,
        Commands::Run(cmd) => cmd.run().await,
        Commands::Usage(cmd) => cmd.run().await,
        Commands::Version(cmd) => cmd.run().await,
    }
}
