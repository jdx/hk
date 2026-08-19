use crate::version as version_lib;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{Result, env, logger, settings::Settings};
use clx::progress::ProgressOutput;
use eyre::WrapErr;

mod agent;
mod builtins;
mod cache;
mod check;
#[cfg(test)]
mod command_effects;
mod completion;
mod config;
mod fix;
mod init;
mod install;
mod mcp;
mod migrate;
mod run;
mod sponsors;
mod test;
mod uninstall;
mod usage;
mod util;
mod validate;
mod version;

#[derive(usage_derive::Cli)]
#[usage(
    name = "hk",
    version = version_lib::version(),
    version_spec = "1.55.0",
    unknown_flags = "error"
)]
struct Cli {
    /// Run as if hk was started in this directory
    #[usage(long, global, value_name = "DIRECTORY", value_hint = ValueHint::DirPath)]
    cd: Option<PathBuf>,
    /// Select human or machine-readable execution output
    #[usage(
        long,
        value_enum,
        default_value_t = crate::structured_output::OutputFormat::default(),
        default = "human"
    )]
    format: crate::structured_output::OutputFormat,
    /// Path to user configuration file (deprecated: use ~/.config/hk/config.pkl or hk.local.pkl)
    #[usage(long, global, value_name = "PATH", hide)]
    hkrc: Option<PathBuf>,
    /// Number of jobs to run in parallel
    #[usage(short, long, global)]
    jobs: Option<NonZero<usize>>,
    /// Profiles to enable/disable
    /// prefix with ! to disable
    /// e.g. --profile slow --profile !fast
    #[usage(short, long, global)]
    profile: Vec<String>,
    /// Shorthand for --profile=slow
    #[usage(short, long, global)]
    slow: bool,
    /// Enables verbose output
    #[usage(short, long, global, count, overrides("--quiet", "--silent"))]
    verbose: u8,
    /// Disables progress output
    #[usage(short, long, global)]
    no_progress: bool,
    /// Suppresses non-essential output (info messages, progress indicators). Failed-step diagnostics are still shown
    #[usage(short, long, global, overrides("--verbose", "--silent"))]
    quiet: bool,
    /// Suppresses all output including warnings. Only errors are shown
    #[usage(long, global, overrides("--quiet", "--verbose"))]
    silent: bool,
    /// Enable tracing spans and performance diagnostics
    #[usage(long, global)]
    trace: bool,
    /// Output in JSON format
    #[usage(long, global)]
    json: bool,
    #[usage(subcommand)]
    command: Commands,
}

/// Re-execute hk in the directory selected by `--cd`.
///
/// `std::process::Command::current_dir` applies the directory at process creation
/// time, avoiding a process-global `set_current_dir` while preserving all of the
/// existing cwd-based config and repository discovery.
fn reexec_for_cd(cd: &Path) -> Result<std::process::ExitStatus> {
    // The child is already rooted correctly, so remove --cd while copying its
    // arguments. This avoids a process-global cwd change and avoids a marker
    // environment variable that would incorrectly affect nested hk commands.
    let mut args = std::env::args_os().skip(1);
    let mut child_args = Vec::new();
    while let Some(arg) = args.next() {
        if arg == "--" {
            child_args.push(arg);
            child_args.extend(args);
            break;
        }
        if arg == "--cd" {
            let _ = args.next();
            continue;
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            if arg.as_bytes().starts_with(b"--cd=") {
                continue;
            }
        }
        #[cfg(not(unix))]
        if arg.to_str().is_some_and(|arg| arg.starts_with("--cd=")) {
            continue;
        }
        child_args.push(arg);
    }

    let status = std::process::Command::new(std::env::current_exe()?)
        .args(child_args)
        .current_dir(cd)
        .status()
        .wrap_err_with(|| format!("failed to run hk in {}", cd.display()))?;
    Ok(status)
}

#[derive(usage_derive::Subcommands)]
enum Commands {
    Agent(Box<agent::Agent>),
    Builtins(Box<builtins::Builtins>),
    #[usage(hide)]
    Cache(Box<cache::Cache>),
    #[usage(alias = "c")]
    Check(Box<check::Check>),
    Completion(Box<completion::Completion>),
    #[usage(alias = "cfg")]
    Config(Box<config::Config>),
    #[usage(alias = "f")]
    Fix(Box<fix::Fix>),
    #[usage(alias = "generate")]
    Init(Box<init::Init>),
    #[usage(alias = "i")]
    Install(Box<install::Install>),
    Mcp(Box<mcp::Mcp>),
    Migrate(Box<migrate::Migrate>),
    #[usage(alias = "r")]
    Run(Box<run::Run>),
    Sponsors(Box<sponsors::Sponsors>),
    Test(Box<test::Test>),
    Uninstall(Box<uninstall::Uninstall>),
    #[usage(hide)]
    Usage(Box<usage::Usage>),
    Util(Box<util::Util>),
    Validate(Box<validate::Validate>),
    Version(Box<version::Version>),
}

impl Commands {
    fn output_format(&self) -> Option<crate::structured_output::OutputFormat> {
        match self {
            Self::Check(command) => command.hook.format,
            Self::Fix(command) => command.hook.format,
            Self::Run(command) => command.output_format(),
            _ => None,
        }
    }
}

pub async fn run() -> Result<Option<std::process::ExitStatus>> {
    let args = Cli::parse();
    if let Some(cd) = &args.cd {
        return reexec_for_cd(cd).map(Some);
    }

    let output_format = args.command.output_format().unwrap_or(args.format);

    // Determine effective log level from CLI flags (env default applied by logger if None)
    let mut level: Option<log::LevelFilter> = None;
    // Derive verbosity overrides first
    Settings::set_cli_snapshot(crate::settings::CliSnapshot {
        hkrc: args.hkrc,
        jobs: args.jobs.map(|n| n.get()),
        profiles: args.profile.clone(),
        slow: args.slow,
        quiet: args.quiet,
        silent: args.silent,
        trace: args.trace,
        output_format,
    });

    if is_ci::cached() || !console::user_attended_stderr() || args.no_progress {
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
        clx::progress::set_output(ProgressOutput::Quiet);
        level = Some(log::LevelFilter::Warn);
    }
    if args.silent {
        clx::progress::set_output(ProgressOutput::Quiet);
        level = Some(log::LevelFilter::Error);
    }

    // Decide tracing enablement and output format
    // Support: --trace, HK_TRACE mode (Text/Json), or effective log level TRACE
    // Structured execution output owns stdout. Keep trace records on stderr so
    // JSON remains a single document and JSONL contains only lifecycle events.
    let json_trace = args.json || *env::HK_JSON || matches!(*env::HK_TRACE, env::TraceMode::Json);

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
        crate::trace::init_tracing(
            json_trace,
            output_format != crate::structured_output::OutputFormat::Human,
        )?;
    }

    // Skip config loading for commands that don't need it
    // - Init: may be run before hk.pkl exists or when existing one is invalid
    // - Migrate: avoid errors during migration with potentially invalid configs
    // - Completion/Usage: shell completion generation shouldn't require valid config
    // - Version: just prints version info
    // - Builtins: just lists compiled-in builtin names, no project config needed
    // - Util: standalone file utilities must not recursively load hk config
    let settings = if matches!(
        args.command,
        Commands::Agent(_)
            | Commands::Builtins(_)
            | Commands::Init(_)
            | Commands::Mcp(_)
            | Commands::Migrate(_)
            | Commands::Completion(_)
            | Commands::Sponsors(_)
            | Commands::Usage(_)
            | Commands::Util(_)
            | Commands::Version(_)
    ) {
        Arc::new(crate::settings::generated::settings::Settings::default())
    } else {
        Settings::try_get().wrap_err("Failed to load configuration")?
    };
    if !settings.terminal_progress {
        clx::osc::configure(settings.terminal_progress);
    }

    // CLI settings snapshot applied above; settings are built from snapshot
    match args.command {
        Commands::Agent(cmd) => cmd.run().await,
        Commands::Builtins(cmd) => cmd.run().await,
        Commands::Cache(cmd) => cmd.run().await,
        Commands::Check(cmd) => cmd.hook.run("check").await,
        Commands::Completion(cmd) => cmd.run().await,
        Commands::Config(cmd) => cmd.run().await,
        Commands::Fix(cmd) => cmd.hook.run("fix").await,
        Commands::Init(cmd) => cmd.run().await,
        Commands::Install(cmd) => cmd.run().await,
        Commands::Mcp(cmd) => cmd.run().await,
        Commands::Migrate(cmd) => cmd.run().await,
        Commands::Run(cmd) => cmd.run().await,
        Commands::Sponsors(cmd) => cmd.run().await,
        Commands::Uninstall(cmd) => cmd.run().await,
        Commands::Usage(cmd) => cmd.run().await,
        Commands::Util(cmd) => cmd.run().await,
        Commands::Validate(cmd) => cmd.run().await,
        Commands::Version(cmd) => cmd.run().await,
        Commands::Test(cmd) => cmd.run().await,
    }?;
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcommands_are_sorted() {
        fn assert_sorted(cmd: &::usage::SpecCommand) {
            let names: Vec<_> = cmd.subcommands.keys().collect();
            let mut sorted = names.clone();
            sorted.sort();
            assert_eq!(
                names, sorted,
                "subcommands below {} are not sorted",
                cmd.name
            );
            for subcmd in cmd.subcommands.values() {
                assert_sorted(subcmd);
            }
        }
        let spec: ::usage::Spec = Cli::to_kdl().parse().expect("derived spec should parse");
        assert_sorted(&spec.cmd);
    }
}
