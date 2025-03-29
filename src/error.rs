use thiserror::Error;

use crate::cmd::CmdResult;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    JoinPaths(#[from] std::env::JoinPathsError),
    #[cfg(unix)]
    #[error(transparent)]
    Nix(#[from] nix::errno::Errno),

    #[error("{} exited with non-zero status: {}\n{}", .0, render_exit_status(.3), .2)]
    ScriptFailed(String, Vec<String>, String, CmdResult),
}

pub type Result<T> = std::result::Result<T, Error>;

fn render_exit_status(result: &CmdResult) -> String {
    match result.status.code() {
        Some(exit_status) => format!("exit code {exit_status}"),
        None => "no exit status".into(),
    }
}
