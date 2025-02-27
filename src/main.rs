#[macro_use]
extern crate log;

pub use eyre::Result;

mod cache;
mod cli;
mod config;
mod env;
mod error;
mod git;
mod glob;
mod hash;
mod logger;
mod settings;
mod step;
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
    cli::run().await
}

#[cfg(unix)]
fn handle_epipe() {
    let mut pipe_stream = signal::unix::signal(SignalKind::pipe()).unwrap();
    tokio::spawn(async move {
        pipe_stream.recv().await;
        debug!("received SIGPIPE");
    });
}
