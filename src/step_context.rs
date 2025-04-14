use crate::{step_depends::StepDepends, tera, ui::style};
use clx::progress::{ProgressJob, ProgressStatus};
use indexmap::IndexMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, Semaphore};

/// Stores all the information/mutexes needed to run a StepJob
pub struct StepContext {
    pub file_locks: IndexMap<PathBuf, Arc<RwLock<()>>>,
    pub semaphore: Arc<Semaphore>,
    pub failed: Arc<Mutex<bool>>,
    pub depends: Arc<StepDepends>,
    pub tctx: tera::Context,
    pub progress: Arc<ProgressJob>,
    pub files_added: Arc<std::sync::Mutex<usize>>,
    pub jobs_total: usize,
    pub jobs_remaining: Arc<std::sync::Mutex<usize>>,
}

impl StepContext {
    pub fn inc_files_added(&self, count: usize) {
        *self.files_added.lock().unwrap() += count;
    }

    pub fn decrement_job_count(&self) {
        *self.jobs_remaining.lock().unwrap() -= 1;
    }

    pub fn update_progress(&self) {
        let files_added = *self.files_added.lock().unwrap();
        let jobs_remaining = *self.jobs_remaining.lock().unwrap();
        let msg = if jobs_remaining > 0 {
            format!(
                "job {} of {}",
                self.jobs_total - jobs_remaining + 1,
                self.jobs_total
            )
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
        self.progress.set_status(if jobs_remaining == 0 {
            ProgressStatus::Done
        } else {
            ProgressStatus::RunningCustom(style::edim("❯").to_string())
        });
    }
}
