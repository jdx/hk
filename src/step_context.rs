use crate::{
    hook::HookContext,
    step::{SharedBatchJobs, Step},
    step_depends::StepDepends,
    ui::style,
};
use clx::progress::{ProgressJob, ProgressStatus};
use indexmap::IndexSet;
use itertools::Itertools;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Stores all the information/mutexes needed to run a StepJob
pub struct StepContext {
    pub step: Arc<Step>,
    pub hook_ctx: Arc<HookContext>,
    pub depends: Arc<StepDepends>,
    /// Batch counts shared by the batched steps of this step's group.
    pub batch_jobs: Arc<SharedBatchJobs>,
    pub progress: Arc<ProgressJob>,
    pub files_added: Arc<Mutex<IndexSet<PathBuf>>>,
    pub jobs_total: Mutex<usize>,
    pub jobs_remaining: Arc<Mutex<usize>>,
    pub status: Mutex<StepStatus>,
}

#[derive(Default, strum::EnumIs)]
pub enum StepStatus {
    #[default]
    Pending,
    Started,
    Aborted,
    Finished,
    Errored(String),
}

impl StepContext {
    pub fn set_jobs_total(&self, count: usize) {
        *self.jobs_total.lock().unwrap() = count;
        *self.jobs_remaining.lock().unwrap() = count;
        // Initialize per-step progress counters only when multiple jobs are present
        if count > 1 {
            self.progress.progress_total(count);
            self.progress.progress_current(0);
            self.progress.prop("show_step_progress", &true);
        } else {
            // Hide per-step bar when not applicable
            self.progress.prop("show_step_progress", &false);
        }
    }

    pub fn increment_job_count(&self, count: usize) {
        if count == 0 {
            return;
        }
        *self.jobs_remaining.lock().unwrap() += count;
        let jobs_total = {
            let mut jobs_total = self.jobs_total.lock().unwrap();
            *jobs_total += count;
            *jobs_total
        };
        if jobs_total > 1 {
            self.progress.progress_total(jobs_total);
            self.progress.prop("show_step_progress", &true);
        }
        self.update_progress();
    }

    pub fn add_files(&self, added_paths: &[PathBuf], created_paths: &[PathBuf]) {
        let mut files_added = self.files_added.lock().unwrap();
        files_added.extend(added_paths.iter().cloned());
        files_added.extend(created_paths.iter().cloned());
        self.hook_ctx.add_files(added_paths, created_paths);
    }

    pub fn decrement_job_count(&self) {
        *self.jobs_remaining.lock().unwrap() -= 1;
        let jobs_total = *self.jobs_total.lock().unwrap();
        let jobs_remaining = *self.jobs_remaining.lock().unwrap();
        if jobs_total > 1 {
            let completed = jobs_total.saturating_sub(jobs_remaining);
            self.progress.progress_current(completed);
        }
        self.update_progress();
    }

    pub fn status_started(&self) {
        let mut status = self.status.lock().unwrap();
        match &*status {
            StepStatus::Pending => {
                *status = StepStatus::Started;
                drop(status);
                let _ = crate::structured_output::emit_step_started(
                    crate::settings::Settings::cli_output_format(),
                    &self.step.name,
                );
                self.update_progress();
            }
            StepStatus::Started
            | StepStatus::Aborted
            | StepStatus::Finished
            | StepStatus::Errored(_) => {}
        }
    }

    pub fn status_aborted(&self) {
        let mut status = self.status.lock().unwrap();
        match &*status {
            StepStatus::Pending | StepStatus::Started => {
                *status = StepStatus::Aborted;
                drop(status);
                self.hook_ctx.mark_step_cancelled(&self.step.name);
                let _ = crate::structured_output::emit_step_completed(
                    crate::settings::Settings::cli_output_format(),
                    &self.step.name,
                    "cancelled",
                );
                self.progress.prop("show_step_progress", &false);
                self.update_progress();
            }
            StepStatus::Aborted | StepStatus::Finished | StepStatus::Errored(_) => {}
        }
    }

    /// End the step because of `err`: cancelled when the error is a
    /// cancellation of the run (see `is_cancelled_run_error`), errored
    /// otherwise.
    pub fn status_error(&self, err: &eyre::Report) {
        if crate::step_group::is_cancelled_run_error(&self.hook_ctx, err) {
            self.status_aborted();
        } else {
            self.status_errored(&err.to_string());
        }
    }

    pub fn status_errored(&self, err: &str) {
        let mut status = self.status.lock().unwrap();
        match &*status {
            StepStatus::Pending | StepStatus::Started => {
                *status = StepStatus::Errored(err.to_string());
                drop(status);
                self.hook_ctx.mark_step_failed(&self.step.name);
                let _ = crate::structured_output::emit_step_completed(
                    crate::settings::Settings::cli_output_format(),
                    &self.step.name,
                    "failed",
                );
                self.progress.prop("show_step_progress", &false);
                self.update_progress();
            }
            StepStatus::Aborted | StepStatus::Finished | StepStatus::Errored(_) => {}
        }
    }

    pub fn status_finished(&self) {
        let mut status = self.status.lock().unwrap();
        match &*status {
            StepStatus::Started => {
                *status = StepStatus::Finished;
                drop(status);
                self.hook_ctx.mark_step_finished(&self.step.name);
                let _ = crate::structured_output::emit_step_completed(
                    crate::settings::Settings::cli_output_format(),
                    &self.step.name,
                    "passed",
                );
                self.progress.prop("show_step_progress", &false);
                self.update_progress();
            }
            StepStatus::Pending
            | StepStatus::Aborted
            | StepStatus::Finished
            | StepStatus::Errored(_) => {}
        }
    }

    fn update_progress(&self) {
        if self.step.hide {
            return;
        }
        let files_added = self.files_added.lock().unwrap();
        let jobs_remaining = *self.jobs_remaining.lock().unwrap();
        let jobs_total = *self.jobs_total.lock().unwrap();
        let msg = if jobs_total > 1 && jobs_remaining > 0 {
            "".to_string() // progress bar is shown instead
        } else if files_added.len() > 3 {
            format!("{} files modified", files_added.len())
        } else if files_added.len() > 1 {
            let len = files_added.len();
            let files = files_added.iter().map(|f| f.display()).join(", ");
            format!("{len} files modified – {files}")
        } else if files_added.len() == 1 {
            let file = files_added.iter().next().unwrap().display();
            format!("1 file modified – {file}")
        } else {
            "".to_string()
        };
        self.progress.prop("message", &msg);
        match &*self.status.lock().unwrap() {
            StepStatus::Pending => {
                self.progress
                    .set_status(ProgressStatus::RunningCustom(style::edim("❯").to_string()));
            }
            StepStatus::Started => {
                self.progress
                    .set_status(ProgressStatus::RunningCustom(style::edim("❯").to_string()));
            }
            StepStatus::Aborted => {
                // Hide all child progress indicators
                for child in self.progress.children() {
                    child.set_status(ProgressStatus::Hide);
                }
                // A user's Ctrl-C stopped this step; otherwise a failing
                // sibling aborted it.
                let message = if crate::step_group::cancelled_by_user(&self.hook_ctx) {
                    "cancelled"
                } else {
                    "aborted"
                };
                self.progress
                    .prop("message", &style::eyellow(message).to_string());
                self.progress
                    .set_status(ProgressStatus::DoneCustom(style::eyellow("⚠").to_string()));
            }
            StepStatus::Finished => {
                self.progress.set_status(ProgressStatus::Done);
            }
            StepStatus::Errored(_err) => {
                self.progress.set_status(ProgressStatus::Failed);
                self.progress
                    .prop("message", &style::ered("ERROR").to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{git::Git, step::RunType};
    use indexmap::IndexMap;
    use std::collections::BTreeSet;
    use tokio::sync::Mutex as AsyncMutex;

    fn step_context(name: &str) -> StepContext {
        let step = Arc::new(Step {
            name: name.to_string(),
            ..Default::default()
        });
        let hook_ctx = Arc::new(HookContext::new(
            Vec::<PathBuf>::new(),
            Arc::new(AsyncMutex::new(Git::new().unwrap())),
            vec![],
            crate::tera::Context::default(),
            crate::step::EXPR_CTX.clone(),
            RunType::Check,
            None,
            IndexMap::new(),
            false,
            BTreeSet::new(),
            BTreeSet::new(),
        ));
        StepContext {
            progress: step.build_step_progress(),
            batch_jobs: Arc::new(SharedBatchJobs::new(vec![step.clone()])),
            step,
            hook_ctx,
            depends: Arc::new(StepDepends::new(&[])),
            files_added: Default::default(),
            jobs_total: Default::default(),
            jobs_remaining: Default::default(),
            status: Default::default(),
        }
    }

    fn cancelled_error() -> eyre::Report {
        eyre::Report::new(ensembler::Error::Cancelled).wrap_err("sleep 60")
    }

    #[test]
    fn cancellation_ends_the_step_cancelled_not_failed() {
        let ctx = step_context("slow");
        ctx.status_started();
        ctx.hook_ctx.failed.cancel(); // Ctrl-C
        ctx.status_error(&cancelled_error());
        assert!(ctx.status.lock().unwrap().is_aborted());
        assert!(
            ctx.hook_ctx
                .cancelled_steps
                .lock()
                .unwrap()
                .contains("slow")
        );
        assert!(ctx.hook_ctx.failed_steps.lock().unwrap().is_empty());
    }

    #[test]
    fn ordinary_error_ends_the_step_failed_even_during_a_cancel() {
        for cancel in [false, true] {
            let ctx = step_context("lint");
            ctx.status_started();
            if cancel {
                ctx.hook_ctx.failed.cancel();
            }
            ctx.status_error(&eyre::eyre!("tool exited 1"));
            assert!(ctx.status.lock().unwrap().is_errored());
            assert!(ctx.hook_ctx.failed_steps.lock().unwrap().contains("lint"));
            assert!(ctx.hook_ctx.cancelled_steps.lock().unwrap().is_empty());
        }
    }

    #[test]
    fn a_cancel_after_a_real_failure_keeps_the_failure() {
        let ctx = step_context("lint");
        ctx.status_started();
        ctx.status_error(&eyre::eyre!("tool exited 1"));
        ctx.hook_ctx.failed.cancel();
        ctx.status_error(&cancelled_error());
        assert!(ctx.status.lock().unwrap().is_errored());
        assert!(ctx.hook_ctx.cancelled_steps.lock().unwrap().is_empty());
    }

    #[test]
    fn cancellation_error_without_a_cancelled_run_is_a_failure() {
        let ctx = step_context("odd");
        ctx.status_started();
        ctx.status_error(&cancelled_error());
        assert!(ctx.status.lock().unwrap().is_errored());
    }
}
