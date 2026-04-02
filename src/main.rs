#[macro_use]
extern crate log;
#[macro_use]
mod output;

use std::{panic, time::Duration};

pub use eyre::Result;

mod builtins;
mod cache;
mod cli;
mod config;
mod diff;
mod env;
mod error;
mod file_rw_locks;
mod file_type;
mod git;
mod git_util;
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

#[cfg(unix)]
use tokio::signal;
#[cfg(unix)]
use tokio::signal::unix::SignalKind;

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

/// Suppress the eyre backtrace for ScriptFailed errors.
/// The output_by_step summary in hook.rs already displayed per-step output,
/// so we just need a clean exit without the full error chain.
fn friendly_error(e: eyre::Report) -> Result<()> {
    if let Some(ensembler::Error::ScriptFailed(err)) =
        e.chain().find_map(|e| e.downcast_ref::<ensembler::Error>())
    {
        write_output_file(&err.3);
        std::process::exit(err.3.status.code().unwrap_or(1));
    }
    Err(e)
}

fn write_output_file(result: &ensembler::CmdResult) {
    let path = env::HK_STATE_DIR.join("output.log");
    let Some(parent) = path.parent() else {
        return;
    };
    if let Err(e) = std::fs::create_dir_all(parent).and_then(|_| {
        let output = console::strip_ansi_codes(&result.combined_output);
        std::fs::write(&path, output.as_ref())
    }) {
        warn!("Error writing output file: {e:?}");
        return;
    }
    eprintln!("\nSee {} for full command output", path.display());
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
