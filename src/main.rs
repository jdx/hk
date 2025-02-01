#[macro_use]
extern crate log;

mod cli;
mod config;
mod env;
mod error;
mod git;
mod hook;
mod logger;
mod plugins;
mod tera;
mod ui;
mod core;

pub use error::Result;
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
