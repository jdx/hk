use crate::{Result, file_rw_locks::Flocks, hook::SkipReason, step::RunType};
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use itertools::Itertools;
use tokio::sync::OwnedSemaphorePermit;

use crate::{env, step::Step, step_context::StepContext, step_locks::StepLocks, tera};
use std::{path::PathBuf, sync::Arc};

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
    pub skip_reason: Option<SkipReason>,
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
            skip_reason: None,
            progress: None,
            semaphore: None,
        }
    }

    pub fn with_workspace_indicator(mut self, workspace_indicator: PathBuf) -> Self {
        self.workspace_indicator = Some(workspace_indicator);
        self
    }

    pub fn workspace_indicator(&self) -> Option<&PathBuf> {
        self.workspace_indicator.as_ref()
    }

    pub fn tctx(&self, base: &tera::Context) -> tera::Context {
        let mut tctx = base.clone();

        tctx.insert("step", &self.step.name);

        // Workspace variables first: `dir` may reference them, and the files
        // below are made relative to the rendered `dir`.
        if let Some(workspace_indicator) = &self.workspace_indicator {
            tctx.with_workspace_indicator(workspace_indicator);
            let workspace_dir = workspace_indicator
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(std::path::Path::new("."));
            tctx.with_workspace_files(self.step.shell_type(), workspace_dir, &self.files);
        }

        // Handle directory stripping for command execution context. A `dir`
        // that fails to render strips nothing; the runner renders it again and
        // reports the error before the command runs.
        let dir = self.step.render_dir(&tctx).ok().flatten();
        // Workspace discovery returns repository-relative paths, but commands
        // with a literal `dir` run from that directory. Keep workspace template
        // paths in the same coordinate system as `files`; subproject merging
        // gives every ordinary step a literal directory, so this also avoids
        // paths such as `ui/ui/tsconfig.json` from a command run in `ui`.
        //
        // A templated `dir` must retain the repository-relative workspace
        // context because the runner renders it again from this same context.
        if !self.step.dir_is_templated()
            && let (Some(workspace_indicator), Some(dir)) = (&self.workspace_indicator, &dir)
            && let Ok(relative_indicator) = workspace_indicator.strip_prefix(dir)
        {
            tctx.with_workspace_indicator(&relative_indicator);
        }
        let command_files = if let Some(dir) = &dir {
            self.files
                .iter()
                .map(|f| f.strip_prefix(dir).unwrap_or(f).to_path_buf())
                .collect::<Vec<_>>()
        } else {
            self.files.clone()
        };

        tctx.with_files(self.step.shell_type(), &command_files);
        tctx
    }

    pub fn build_progress(&self, ctx: &StepContext) -> Arc<ProgressJob> {
        let job = ProgressJobBuilder::new()
            .prop("name", &self.step.name)
            .prop("files", &self.files.iter().map(|f| f.display()).join(" "))
            .body(
                "{{spinner()}} {% if ensembler_cmd %}{{ensembler_cmd | flex}}{% if ensembler_stdout %}\n{{ensembler_stdout | flex}}{% endif %}{% else %}{{message | flex}}{% endif %}"
            )
            // Text mode (CI / piped stderr) keeps the full message — the
            // log viewer handles wrapping, and a 60-char truncate just hides
            // the diagnostic detail callers actually need to debug a
            // failure. The UI-mode `body` above still uses `flex` because
            // the in-place renderer needs bounded line widths.
            .body_text(Some(
                "{% if ensembler_stdout %}  {{name}} – {{ensembler_stdout}}{% elif message %}{{spinner()}} {{name}} – {{message}}{% endif %}".to_string(),
            ))
            .status(ProgressStatus::Hide)
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
            // A command may finish successfully before an orchestration-level
            // postcondition detects a failure (for example, disagreement
            // between a file-listing check and a focused diagnostic check).
            StepJobStatus::Pending | StepJobStatus::Started(_) | StepJobStatus::Finished => {}
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
            skip_reason: self.skip_reason.clone(),
            workspace_indicator: self.workspace_indicator.clone(),
            status: StepJobStatus::Pending,
            progress: self.progress.clone(),
            semaphore: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_templates_are_relative_to_literal_dir() {
        let step = Arc::new(Step {
            name: "tsc".to_string(),
            dir: Some("ui".to_string()),
            ..Default::default()
        });
        let job = StepJob::new(step, vec![PathBuf::from("ui/src/main.ts")], RunType::Check)
            .with_workspace_indicator(PathBuf::from("ui/tsconfig.json"));

        let tctx = job.tctx(&tera::Context::default());

        assert_eq!(tera::render("{{workspace}}", &tctx).unwrap(), ".");
        assert_eq!(
            tera::render("{{workspace_indicator}}", &tctx).unwrap(),
            "tsconfig.json"
        );
        assert_eq!(tera::render("{{files}}", &tctx).unwrap(), "src/main.ts");
    }

    #[test]
    fn templated_dir_keeps_repository_relative_workspace_context() {
        let step = Arc::new(Step {
            name: "go-vet".to_string(),
            dir: Some("{{workspace}}".to_string()),
            ..Default::default()
        });
        let job = StepJob::new(
            step,
            vec![PathBuf::from("pkgs/api/main.go")],
            RunType::Check,
        )
        .with_workspace_indicator(PathBuf::from("pkgs/api/go.mod"));

        let tctx = job.tctx(&tera::Context::default());

        assert_eq!(tera::render("{{workspace}}", &tctx).unwrap(), "pkgs/api");
        assert_eq!(
            tera::render("{{workspace_indicator}}", &tctx).unwrap(),
            "pkgs/api/go.mod"
        );
    }
}
