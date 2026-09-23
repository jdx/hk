#[macro_use]
extern crate log;
use std::{
    ffi::OsString,
    io::{self, Write},
    panic, thread,
    time::Duration,
};

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
        return write_builtins(io::stdout().lock());
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

fn write_builtins(mut writer: impl Write) -> Result<()> {
    for builtin in builtins::BUILTINS {
        if let Err(err) = writeln!(writer, "{builtin}") {
            if err.kind() == io::ErrorKind::BrokenPipe {
                return Ok(());
            }
            return Err(err.into());
        }
    }
    Ok(())
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
        // `print!`/`eprint!` panic when the reader has gone away (`hk --version | true`, a
        // cancelled completion). Release builds abort on panic, so without this every closed
        // pipe dumps core. Leave the way a process killed by SIGPIPE would instead, and before
        // flushing progress, which would write to the same closed pipe.
        if panic_info
            .payload_as_str()
            .is_some_and(is_broken_pipe_print)
        {
            exit_on_broken_pipe();
        }
        clx::progress::flush();
        default_panic(panic_info);
    }));
}

/// Whether a panic message is std's report of a print macro writing to a closed pipe.
///
/// Compared against the rendered `EPIPE` rather than a literal, because that text comes from
/// the platform's `strerror` and differs between platforms.
fn is_broken_pipe_print(message: &str) -> bool {
    let Some(error) = ["failed printing to stdout: ", "failed printing to stderr: "]
        .iter()
        .find_map(|prefix| message.strip_prefix(prefix))
    else {
        return false;
    };
    error == broken_pipe_error().to_string()
}

/// The OS error a write to a closed pipe fails with, as std renders it in a print panic.
fn broken_pipe_error() -> io::Error {
    #[cfg(unix)]
    let code = libc::EPIPE;
    // ERROR_NO_DATA, which std maps to `ErrorKind::BrokenPipe`.
    #[cfg(windows)]
    let code = 232;
    io::Error::from_raw_os_error(code)
}

/// Terminate as if killed by SIGPIPE: no panic report, no core dump, and a status (141 in a
/// shell) that does not read as success, since a check may have been failing when the pipe
/// closed.
fn exit_on_broken_pipe() -> ! {
    #[cfg(unix)]
    // SAFETY: restoring the default disposition and raising a signal on the current process
    // have no memory-safety preconditions.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        libc::raise(libc::SIGPIPE);
    }
    std::process::exit(128 + 13);
}

#[cfg(test)]
mod tests {
    use super::{
        broken_pipe_error, is_bare_builtins_invocation, is_broken_pipe_print,
        runtime_worker_threads, write_builtins,
    };
    use std::ffi::OsString;
    use std::io::{self, Write};

    #[test]
    fn bare_builtins_can_skip_runtime_and_command_tree_setup() {
        assert!(is_bare_builtins_invocation(
            ["builtins"].into_iter().map(OsString::from)
        ));
        assert!(!is_bare_builtins_invocation(
            ["builtins", "--quiet"].into_iter().map(OsString::from)
        ));
    }

    struct FailingWriter(io::ErrorKind);

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn bare_builtins_treats_broken_pipe_as_success() {
        write_builtins(FailingWriter(io::ErrorKind::BrokenPipe)).unwrap();
        assert!(write_builtins(FailingWriter(io::ErrorKind::Other)).is_err());
    }

    #[test]
    fn recognizes_only_broken_pipe_print_panics() {
        let epipe = broken_pipe_error();
        assert_eq!(epipe.kind(), io::ErrorKind::BrokenPipe);
        assert!(is_broken_pipe_print(&format!(
            "failed printing to stdout: {epipe}"
        )));
        assert!(is_broken_pipe_print(&format!(
            "failed printing to stderr: {epipe}"
        )));
        let other = io::Error::from_raw_os_error(28);
        assert!(!is_broken_pipe_print(&format!(
            "failed printing to stdout: {other}"
        )));
        assert!(!is_broken_pipe_print(&format!("{epipe}")));
    }

    #[test]
    fn runtime_workers_are_bounded() {
        assert_eq!(runtime_worker_threads(0), 1);
        assert_eq!(runtime_worker_threads(4), 4);
        assert_eq!(runtime_worker_threads(32), 16);
    }
}
