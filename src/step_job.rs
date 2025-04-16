use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior};
use itertools::Itertools;

use crate::{env, step::Step, step_context::StepContext, step_locks::StepLocks, tera};
use std::{path::PathBuf, sync::Arc};

use crate::step::RunType;

/// Represents a single work item for the scheduler
///
/// A single step may have multiple jobs associated with it, such as:
///
/// * Multiple workspace_indicators to run step in different workspaces
/// * Batch step that needs to run multiple batches of different files
#[derive(Debug)]
pub struct StepJob {
    pub step: Arc<Step>,
    pub files: Vec<PathBuf>,
    pub run_type: RunType,
    pub check_first: bool,
    pub progress: Arc<ProgressJob>,
    workspace_indicator: Option<PathBuf>,

    pub status: StepJobStatus,
}

#[derive(Debug, strum::EnumIs, strum::Display)]
pub enum StepJobStatus {
    Pending,
    // Ready(StepLocks),
    Started(StepLocks),
    // Finished,
    // Errored,
}

// impl StepJobStatus {
//     pub fn is_complete(&self) -> bool {
//         matches!(self, StepJobStatus::Finished | StepJobStatus::Errored)
//     }
// }

impl StepJob {
    pub fn new(step: Arc<Step>, files: Vec<PathBuf>, run_type: RunType) -> Self {
        Self {
            files,
            run_type,
            workspace_indicator: None,
            check_first: *env::HK_CHECK_FIRST
                && step.check_first
                && step.fix.is_some()
                && (step.check.is_some()
                    || step.check_diff.is_some()
                    || step.check_list_files.is_some())
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

    pub fn build_progress(&self, ctx: &StepContext) -> Arc<ProgressJob> {
        let job = ProgressJobBuilder::new()
            .prop("name", &self.step.name)
            .prop("files", &self.files.iter().map(|f| f.display()).join(" "))
            .body(vec![
                // TODO: truncate properly
                "{{spinner()}} {% if ensembler_cmd %}{{ensembler_cmd | flex}}\n{{ensembler_stdout | flex}}{% else %}{{message | flex}}{% endif %}"
                    .to_string(),
            ])
            .body_text(Some(vec![
                "{% if ensembler_stdout %}  {{name}} – {{ensembler_stdout}}{% elif message %}{{spinner()}} {{name}} – {{message}}{% endif %}".to_string(),
            ]))
            .on_done(ProgressJobDoneBehavior::Hide)
            .build();
        ctx.progress.add(job)
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
