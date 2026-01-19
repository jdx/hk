//! Progress tracking for step execution.
//!
//! Creates and manages progress bars/spinners for steps during execution.

use crate::env;
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use std::sync::Arc;

use super::types::Step;

impl Step {
    /// Build a progress tracker for this step.
    ///
    /// Creates a progress job with appropriate templates for displaying
    /// step status, file counts, and messages.
    pub(crate) fn build_step_progress(&self) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body("{{spinner()}} {{name | flex}} {% if show_step_progress %}{{progress_bar(width=20)}} {{cur}}/{{total}}{% endif %}{% if message %} – {{message | flex}}{% elif files %} – {{files}}{% endif %}")
            .body_text(Some(
                "{{spinner()}} {{name}}{% if show_step_progress %}  {{progress_bar(width=20)}} {{cur}}/{{total}}{% endif %}{% if message %} – {{message}}{% elif files %} – {{files}}{% endif %}",
            ))
            .prop("name", &self.name)
            .prop("files", &0)
            .status(ProgressStatus::Hide)
            .on_done(if *env::HK_HIDE_WHEN_DONE {
                ProgressJobDoneBehavior::Hide
            } else {
                ProgressJobDoneBehavior::Keep
            })
            .start()
    }
}
