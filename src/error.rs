//pub use std::error::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("check list failed: {source}")]
    CheckListFailed {
        #[source]
        source: eyre::Error,
        stdout: String,
        stderr: String,
        combined: String,
    },

    #[error("{step}: file-listing check failed but focused check succeeded")]
    FocusedCheckMismatch { step: String },
}

pub fn is_command_failure(error: &eyre::Report) -> bool {
    error.chain().any(|error| {
        matches!(
            error.downcast_ref::<Error>(),
            Some(Error::CheckListFailed { .. } | Error::FocusedCheckMismatch { .. })
        ) || matches!(
            error.downcast_ref::<ensembler::Error>(),
            Some(ensembler::Error::ScriptFailed(_))
        )
    })
}
