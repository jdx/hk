use crate::{Result, file_rw_locks::Flocks};
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use itertools::Itertools;
use tokio::sync::OwnedSemaphorePermit;

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
    pub progress: Option<Arc<ProgressJob>>,
    pub semaphore: Option<OwnedSemaphorePermit>,
    workspace_indicator: Option<PathBuf>,

    pub status: StepJobStatus,
}

#[derive(Debug, strum::EnumIs, strum::Display)]
pub enum StepJobStatus {
    Pending,
    Started(StepLocks),
    Finished,
    Errored(String),
}

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
            progress: None,
            semaphore: None,
        }
    }

    pub fn with_workspace_indicator(mut self, workspace_indicator: PathBuf) -> Self {
        self.workspace_indicator = Some(workspace_indicator);
        self
    }

    pub fn tctx(&self, base: &tera::Context) -> tera::Context {
        let mut tctx = base.clone();
        tctx.with_files(self.step.shell_type(), &self.files);
        if let Some(workspace_indicator) = &self.workspace_indicator {
            tctx.with_workspace_indicator(workspace_indicator);
        }
        tctx
    }

    pub fn build_progress(&self, ctx: &StepContext) -> Arc<ProgressJob> {
        let job = ProgressJobBuilder::new()
            .prop("name", &self.step.name)
            .prop("files", &self.files.iter().map(|f| f.display()).join(" "))
            .body(
                // TODO: truncate properly
                "{{spinner()}} {% if ensembler_cmd %}{{ensembler_cmd | flex}}\n{{ensembler_stdout | flex}}{% else %}{{message | flex}}{% endif %}"
            )
            .body_text(Some(
                "{% if ensembler_stdout %}  {{name}} – {{ensembler_stdout}}{% elif message %}{{spinner()}} {{name}} – {{message}}{% endif %}".to_string(),
            ))
            .on_done(ProgressJobDoneBehavior::Hide)
            .build();
        ctx.progress.add(job)
    }

    pub async fn status_start(
        &mut self,
        ctx: &StepContext,
        semaphore: OwnedSemaphorePermit,
    ) -> Result<()> {
        match &self.status {
            StepJobStatus::Pending => {}
            StepJobStatus::Started(_) => {
                return Ok(());
            }
            _ => unreachable!("invalid status: {:?}", self.status),
        }
        let flocks = self.flocks(ctx).await;
        self.status = StepJobStatus::Started(StepLocks::new(flocks, semaphore));
        ctx.status_started();
        if let Some(progress) = &mut self.progress {
            progress.set_status(ProgressStatus::Running);
        }
        Ok(())
    }

    pub fn status_finished(&mut self) -> Result<()> {
        match &mut self.status {
            StepJobStatus::Started(_) => {}
            _ => unreachable!("invalid status: {:?}", self.status),
        }
        self.status = StepJobStatus::Finished;
        if let Some(progress) = &mut self.progress {
            progress.set_status(ProgressStatus::Done);
        }
        Ok(())
    }

    pub async fn status_errored(&mut self, ctx: &StepContext, err: String) -> Result<()> {
        match &mut self.status {
            StepJobStatus::Pending | StepJobStatus::Started(_) => {}
            _ => unreachable!("invalid status: {:?}", self.status),
        }
        self.status = StepJobStatus::Errored(err.to_string());
        if let Some(progress) = &mut self.progress {
            progress.prop("message", &err);
            progress.set_status(ProgressStatus::Failed);
        }
        ctx.status_errored(&err);
        Ok(())
    }

    async fn flocks(&self, ctx: &StepContext) -> Flocks {
        if self.step.stomp {
            Default::default()
        } else if self.run_type == RunType::Fix {
            ctx.hook_ctx.file_locks.write_locks(&self.files).await
        } else {
            ctx.hook_ctx.file_locks.read_locks(&self.files).await
        }
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
            semaphore: None,
        }
    }
}
