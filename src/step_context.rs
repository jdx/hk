use crate::{hook::HookContext, step::Step, step_depends::StepDepends, ui::style};
use clx::progress::{ProgressJob, ProgressStatus};
use indexmap::IndexSet;
use itertools::Itertools;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Stores all the information/mutexes needed to run a StepJob
pub struct StepContext {
    pub step: Step,
    pub hook_ctx: Arc<HookContext>,
    pub depends: Arc<StepDepends>,
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
                self.progress.prop("show_step_progress", &false);
                self.update_progress();
            }
            StepStatus::Aborted | StepStatus::Finished | StepStatus::Errored(_) => {}
        }
    }

    pub fn status_errored(&self, err: &str) {
        let mut status = self.status.lock().unwrap();
        match &*status {
            StepStatus::Pending | StepStatus::Started => {
                *status = StepStatus::Errored(err.to_string());
                drop(status);
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
                // Keep hidden until started - don't show spinners for queued work
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
                self.progress
                    .prop("message", &style::eyellow("aborted").to_string());
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
