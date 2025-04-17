use crate::{hook::HookContext, step_depends::StepDepends, ui::style};
use clx::progress::{ProgressJob, ProgressStatus};
use std::{path::PathBuf, sync::Arc};

/// Stores all the information/mutexes needed to run a StepJob
pub struct StepContext {
    // pub step: Arc<Step>,
    pub hook_ctx: Arc<HookContext>,
    pub depends: Arc<StepDepends>,
    pub progress: Arc<ProgressJob>,
    pub files_added: Arc<std::sync::Mutex<usize>>,
    pub jobs_total: std::sync::Mutex<usize>,
    pub jobs_remaining: Arc<std::sync::Mutex<usize>>,
    pub status: std::sync::Mutex<StepStatus>,
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
    }

    pub fn add_files(&self, files: &[PathBuf]) {
        *self.files_added.lock().unwrap() += files.len();
        self.hook_ctx
            .file_locks
            .add_files(files.iter().map(|p| p.to_path_buf()));
    }

    pub fn decrement_job_count(&self) {
        *self.jobs_remaining.lock().unwrap() -= 1;
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
                self.update_progress();
            }
            StepStatus::Aborted | StepStatus::Finished | StepStatus::Errored(_) => {}
        }
    }

    pub fn status_finished(&self) {
        let mut status = self.status.lock().unwrap();
        match &*status {
            StepStatus::Pending | StepStatus::Started => {
                *status = StepStatus::Finished;
                drop(status);
                self.update_progress();
            }
            StepStatus::Aborted | StepStatus::Finished | StepStatus::Errored(_) => {}
        }
    }

    fn update_progress(&self) {
        let files_added = *self.files_added.lock().unwrap();
        let jobs_remaining = *self.jobs_remaining.lock().unwrap();
        let jobs_total = *self.jobs_total.lock().unwrap();
        let msg = if jobs_total > 1 && jobs_remaining > 0 {
            format!("job {} of {}", jobs_total - jobs_remaining + 1, jobs_total)
        } else if files_added > 0 {
            format!(
                "{} file{} modified",
                files_added,
                if files_added == 1 { "" } else { "s" }
            )
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
                self.progress.set_status(ProgressStatus::Hide);
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
