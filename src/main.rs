#[macro_use]
extern crate log;
#[macro_use]
mod output;

use std::{ffi::OsString, panic, thread, time::Duration};

pub use eyre::Result;

mod builtins;
mod cache;
mod cli;
mod config;
mod diagnostics;
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
mod mise_env;
mod plan;
mod settings;
mod step;
mod step_context;
mod step_depends;
mod step_group;
mod step_job;
mod step_locks;
mod step_test;
mod structured_output;
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

fn main() -> Result<()> {
    if is_bare_builtins_invocation(std::env::args_os().skip(1)) {
        for builtin in builtins::BUILTINS {
            println!("{builtin}");
        }
        return Ok(());
    }
    let worker_threads = runtime_worker_threads(
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
    );
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(worker_threads)
        .build()?
        .block_on(async_main())
}

fn is_bare_builtins_invocation(mut args: impl Iterator<Item = OsString>) -> bool {
    args.next().is_some_and(|arg| arg == "builtins") && args.next().is_none()
}

async fn async_main() -> Result<()> {
    #[cfg(unix)]
    handle_epipe();
    clx::progress::set_interval(Duration::from_millis(200));
    handle_panic();
    let result = cli::run().await;
    clx::progress::flush();
    match result {
        Ok(Some(status)) => std::process::exit(status.code().unwrap_or(1)),
        Ok(None) => Ok(()),
        Err(e) if !log::log_enabled!(log::Level::Debug) => friendly_error(e),
        Err(e) => Err(e),
    }
}

fn runtime_worker_threads(available_parallelism: usize) -> usize {
    available_parallelism.clamp(1, 16)
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
    let path = &*env::HK_OUTPUT_FILE;
    let create_parent = if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
    } else {
        Ok(())
    };
    if let Err(e) = create_parent.and_then(|_| {
        let output = console::strip_ansi_codes(&result.combined_output);
        std::fs::write(path, output.as_ref())
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

#[cfg(test)]
mod tests {
    use super::{is_bare_builtins_invocation, runtime_worker_threads};
    use std::ffi::OsString;

    #[test]
    fn bare_builtins_can_skip_runtime_and_command_tree_setup() {
        assert!(is_bare_builtins_invocation(
            ["builtins"].into_iter().map(OsString::from)
        ));
        assert!(!is_bare_builtins_invocation(
            ["builtins", "--quiet"].into_iter().map(OsString::from)
        ));
    }

    #[test]
    fn runtime_workers_are_bounded() {
        assert_eq!(runtime_worker_threads(0), 1);
        assert_eq!(runtime_worker_threads(4), 4);
        assert_eq!(runtime_worker_threads(32), 16);
    }
}
