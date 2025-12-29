#[macro_use]
extern crate log;

use std::{panic, time::Duration};

pub use eyre::Result;

mod cache;
mod cli;
mod config;
mod env;
mod error;
mod file_rw_locks;
mod file_type;
mod git;
mod glob;
mod hash;
mod hook;
mod hook_options;
mod logger;
mod merge;
mod settings;
mod step;
mod step_context;
mod step_depends;
mod step_group;
mod step_job;
mod step_locks;
mod step_test;
mod tera;
mod test_runner;
mod timings;
mod trace;
mod ui;
mod version;

use std::fs;

#[cfg(unix)]
use tokio::signal;
#[cfg(unix)]
use tokio::signal::unix::SignalKind;
use ui::style;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(unix)]
    handle_epipe();
    clx::progress::set_interval(Duration::from_millis(200));
    handle_panic();
    let result = cli::run().await;
    clx::progress::flush();
    match result {
        Err(e) if !log::log_enabled!(log::Level::Debug) => friendly_error(e),
        r => r,
    }
}

fn friendly_error(e: eyre::Report) -> Result<()> {
    if let Some(ensembler::Error::ScriptFailed(err)) =
        e.chain().find_map(|e| e.downcast_ref::<ensembler::Error>())
    {
        handle_script_failed(&err.0, &err.1, &err.2, &err.3);
    }
    Err(e)
}

fn handle_script_failed(bin: &str, args: &[String], output: &str, result: &ensembler::CmdResult) {
    clx::progress::flush();
    let mut cmd = format!("{} {}", bin, args.join(" "));
    if cmd.starts_with("sh -o errexit -c ") {
        cmd = cmd[17..].to_string();
    }
    eprintln!("{}\n{output}", style::ered(format!("Error running {cmd}")));
    if let Err(e) = write_output_file(result) {
        eprintln!("Error writing output file: {e:?}");
    }
    std::process::exit(result.status.code().unwrap_or(1));
}

fn write_output_file(result: &ensembler::CmdResult) -> Result<()> {
    let path = env::HK_STATE_DIR.join("output.log");
    std::fs::create_dir_all(path.parent().unwrap())?;
    let output = console::strip_ansi_codes(&result.combined_output);
    std::fs::write(&path, output.to_string())?;
    eprintln!("\nSee {} for full command output", path.display());
    Ok(())
}

#[cfg(unix)]
fn handle_epipe() {
    let mut pipe_stream = signal::unix::signal(SignalKind::pipe()).unwrap();
    tokio::spawn(async move {
        pipe_stream.recv().await;
        debug!("received SIGPIPE");
    });
}

fn handle_panic() {
    let default_panic = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        clx::progress::flush();
        default_panic(panic_info);
    }));
}
