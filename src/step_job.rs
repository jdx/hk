use clx::progress::{ProgressJob, ProgressJobBuilder};

use crate::{env, step_locks::StepLocks, tera};
use std::{path::PathBuf, sync::Arc};

use crate::step::{RunType, Step};

/// Represents a single work item for the scheduler
///
/// A single step may have multiple jobs associated with it, such as:
///
/// * Multiple workspace_indicators to run step in different workspaces
/// * Batch step that needs to run multiple batches of different files
pub struct StepJob {
    pub step: Arc<Step>,
    pub files: Vec<PathBuf>,
    pub run_type: RunType,
    pub check_first: bool,
    pub progress: Arc<ProgressJob>,
    workspace_indicator: Option<PathBuf>,

    pub status: StepJobStatus,
}

#[derive(strum::EnumIs, strum::Display)]
pub enum StepJobStatus {
    Pending,
    // Ready(StepLocks),
    Started(StepLocks),
    // Finished,
    // Errored,
}

impl StepJob {
    pub fn new(step: Arc<Step>, files: Vec<PathBuf>, run_type: RunType) -> Self {
        Self {
            files,
            run_type,
            workspace_indicator: None,
            check_first: *env::HK_CHECK_FIRST
                && step.check_first
                && matches!(run_type, RunType::Fix),
            step,
            status: StepJobStatus::Pending,
            progress: Arc::new(ProgressJobBuilder::new().build()),
        }
    }

    pub fn with_workspace_indicator(mut self, workspace_indicator: PathBuf) -> Self {
        let workspace_dir = workspace_indicator.parent().unwrap();
        self.files = self
            .files
            .iter()
            .filter(|f| f.starts_with(workspace_dir))
            .cloned()
            .collect();
        self.workspace_indicator = Some(workspace_indicator);
        self
    }

    pub fn tctx(&self, base: &tera::Context) -> tera::Context {
        let mut tctx = base.clone();
        tctx.with_files(&self.files);
        if let Some(workspace_indicator) = &self.workspace_indicator {
            tctx.with_workspace_indicator(workspace_indicator);
        }
        tctx
    }
}

impl Clone for StepJob {
    fn clone(&self) -> Self {
        Self {
            step: self.step.clone(),
            files: self.files.clone(),
            run_type: self.run_type,
            check_first: self.check_first,
            workspace_indicator: self.workspace_indicator.clone(),
            status: StepJobStatus::Pending,
            progress: self.progress.clone(),
        }
    }
}
