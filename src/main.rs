#[macro_use]
extern crate log;

use std::time::Duration;

pub use eyre::Result;

mod cache;
mod cli;
mod config;
mod env;
mod error;
mod git;
mod glob;
mod hash;
mod hook_options;
mod logger;
mod settings;
mod step;
mod step_context;
mod step_depends;
mod step_job;
mod step_locks;
mod step_queue;
mod step_response;
mod step_scheduler;
mod tera;
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
    match cli::run().await {
        Ok(()) => {
            clx::progress::flush();
            Ok(())
        }
        Err(e) => {
            clx::progress::flush();
            return Err(e);
        }
    }
}

#[cfg(unix)]
fn handle_epipe() {
    let mut pipe_stream = signal::unix::signal(SignalKind::pipe()).unwrap();
    tokio::spawn(async move {
        pipe_stream.recv().await;
        debug!("received SIGPIPE");
    });
}
