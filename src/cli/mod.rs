use crate::{logger, Result};
use clap::Parser;

mod cache;
mod format;
mod init;
mod install;
mod run;
mod config;
mod generate;

#[derive(Debug, clap::Parser)]
#[clap(name = "hk", version = env!("CARGO_PKG_VERSION"), about = env!("CARGO_PKG_DESCRIPTION"))]
struct Cli {
    /// Enables verbose output
    #[clap(short, long, global = true, overrides_with_all = ["quiet", "silent"])]
    verbose: bool,
    /// Suppresses output
    #[clap(short, long, global = true, overrides_with_all = ["verbose", "silent"])]
    quiet: bool,
    /// Suppresses all output
    #[clap(short, long, global = true, overrides_with_all = ["quiet", "verbose"])]
    silent: bool,
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    Cache(cache::Cache),
    Config(config::Config),
    Format(format::Format),
    Generate(generate::Generate),
    Init(init::Init),
    Install(install::Install),
    Run(run::Run),
}

pub async fn run() -> Result<()> {
    let args = Cli::parse();
    let mut level = None;
    if args.verbose || log::log_enabled!(log::Level::Debug) {
        ensembler::MultiProgressReport::set_output_type(ensembler::OutputType::Verbose);
        level = Some(log::LevelFilter::Debug);
    }
    if args.quiet {
        ensembler::MultiProgressReport::set_output_type(ensembler::OutputType::Quiet);
        level = Some(log::LevelFilter::Warn);
    }
    if args.silent {
        ensembler::MultiProgressReport::set_output_type(ensembler::OutputType::Quiet);
        level = Some(log::LevelFilter::Error);
    }
    logger::init(level);
    match args.command {
        Commands::Cache(cmd) => cmd.run().await,
        Commands::Config(cmd) => cmd.run().await,
        Commands::Format(cmd) => cmd.run().await,
        Commands::Generate(cmd) => cmd.run().await,
        Commands::Init(cmd) => cmd.run().await,
        Commands::Install(cmd) => cmd.run().await,
        Commands::Run(cmd) => cmd.run().await,
    }
}
