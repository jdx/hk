use thiserror::Error;
use std::process::ExitStatus;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    JoinPaths(#[from] std::env::JoinPathsError),
    #[error(transparent)]
    Nix(#[from] nix::errno::Errno),

    #[error("{} exited with non-zero status: {}", .0, render_exit_status(.1))]
    ScriptFailed(String, Option<ExitStatus>),
}

pub type Result<T> = std::result::Result<T, Error>;

fn render_exit_status(exit_status: &Option<ExitStatus>) -> String {
    match exit_status.and_then(|s| s.code()) {
        Some(exit_status) => format!("exit code {exit_status}"),
        None => "no exit status".into(),
    }
}
