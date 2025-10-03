//pub use std::error::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("check list failed: {source}")]
    CheckListFailed {
        #[source]
        source: eyre::Error,
        stdout: String,
        stderr: String,
    },
}
