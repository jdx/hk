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

/// Whether the error is a command being cancelled, as opposed to failing.
pub fn is_cancellation(error: &eyre::Report) -> bool {
    error.chain().any(|error| {
        matches!(
            error.downcast_ref::<ensembler::Error>(),
            Some(ensembler::Error::Cancelled)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_found_through_context() {
        let err = eyre::Report::new(ensembler::Error::Cancelled).wrap_err("some command");
        assert!(is_cancellation(&err));
        assert!(!is_command_failure(&err));
        assert!(!is_cancellation(&eyre::eyre!("other")));
    }
}
