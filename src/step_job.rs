use crate::{
    Result,
    file_rw_locks::Flocks,
    hook::SkipReason,
    step::{CommandEffect, RunType},
};
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use tokio::sync::{OwnedRwLockWriteGuard, OwnedSemaphorePermit};

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
    /// The mode requested by the user, retained while check-first temporarily
    /// changes `run_type` to select a read-only command.
    requested_run_type: RunType,
    pub check_first: bool,
    pub skip_reason: Option<SkipReason>,
    pub progress: Option<Arc<ProgressJob>>,
    pub semaphore: Option<OwnedSemaphorePermit>,
    workspace_indicator: Option<PathBuf>,
    /// Set once this job has computed a patch under read locks and traded
    /// them for write locks, so any rerun of `check_diff` keeps writers out.
    pub diff_needs_write_locks: bool,
    /// Set while `check` runs again after a patch applied, for a step with
    /// `check_after_diff`.
    pub rechecking_after_diff: bool,

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
            requested_run_type: run_type,
            workspace_indicator: None,
            check_first: *env::HK_CHECK_FIRST
                && step.check_first()
                && (step.fix.is_some() || step.check_diff.is_some())
                && (step.check.is_some()
                    || step.check_diff.is_some()
                    || step.check_list_files.is_some())
                && matches!(run_type, RunType::Fix),
            step,
            status: StepJobStatus::Pending,
            skip_reason: None,
            progress: None,
            semaphore: None,
            diff_needs_write_locks: false,
            rechecking_after_diff: false,
        }
    }

    /// Whether this job is running `check_diff` first in fix mode under read
    /// locks, so that steps reading the same files run alongside it. It takes
    /// write locks only if there is a patch to apply
    /// (see [`Self::relock_for_write`]).
    pub fn diffs_under_read_locks(&self) -> bool {
        self.check_first
            && self.run_type == RunType::Check
            && self.requested_run_type == RunType::Fix
            && !self.diff_needs_write_locks
            && self.step.diffs_under_read_locks()
    }

    /// Trade this job's read locks for write locks on `self.files`, keeping
    /// its job slot unless it has to wait for them.
    ///
    /// The read locks are released first, so this never waits while holding
    /// locks. Another step may write the files in between; callers compare
    /// write counts to find out.
    pub async fn relock_for_write(&mut self, ctx: &StepContext) -> Result<()> {
        let status = std::mem::replace(&mut self.status, StepJobStatus::Pending);
        let StepJobStatus::Started(locks) = status else {
            unreachable!("relocking a job that is not running: {status:?}")
        };
        self.diff_needs_write_locks = true;
        let semaphore = locks.into_semaphore();
        // The command that printed the patch has finished, and its progress
        // row with it. Applying the patch runs no command, so the row stays
        // finished: marking it running again would leave its spinner and
        // command on screen once the step is done. Diffing again runs the
        // command anew, with a fresh row.
        self.take_locks(ctx, Some(semaphore)).await?;
        Ok(())
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

    /// Take this job's file locks and a job slot, then mark it running.
    ///
    /// `semaphore` is a slot the caller already holds, if any. A job that has
    /// to wait for another step to release its files gives that slot up while
    /// it waits, so a job that can run takes it instead of every slot sitting
    /// behind one slow fixer.
    pub async fn status_start(
        &mut self,
        ctx: &StepContext,
        semaphore: Option<OwnedSemaphorePermit>,
    ) -> Result<()> {
        if !self.take_locks(ctx, semaphore).await? {
            return Ok(());
        }
        ctx.status_started();
        if let Some(progress) = &mut self.progress {
            progress.set_status(ProgressStatus::Running);
        }
        Ok(())
    }

    /// Take this job's file locks and a job slot, as [`Self::status_start`]
    /// does, without marking the job or its progress running. Returns false
    /// if the job already holds them.
    async fn take_locks(
        &mut self,
        ctx: &StepContext,
        mut semaphore: Option<OwnedSemaphorePermit>,
    ) -> Result<bool> {
        match &self.status {
            StepJobStatus::Pending => {}
            StepJobStatus::Started(_) => {
                return Ok(false);
            }
            _ => unreachable!("invalid status: {:?}", self.status),
        }
        let flocks = match self.try_flocks(ctx) {
            Some(flocks) => flocks,
            None => {
                semaphore = None;
                self.flocks(ctx).await
            }
        };
        // Every job holding a slot also holds its locks, so the slot holders
        // always finish and this wait cannot deadlock.
        let semaphore = match semaphore {
            Some(semaphore) => semaphore,
            None => ctx.hook_ctx.semaphore().await,
        };
        // Wait for file locks and a job slot before taking shared access, so
        // a waiting command cannot hold up an active patch transaction.
        let command_guard = ctx.hook_ctx.diff_lock.clone().read_owned().await;
        self.status = StepJobStatus::Started(StepLocks::new(flocks, semaphore, command_guard));
        Ok(true)
    }

    /// Exclude commands, staging, and other patches through apply and rollback.
    /// Never acquire more file locks while holding this exclusive guard.
    pub async fn lock_diff(&mut self, ctx: &StepContext) -> Result<OwnedRwLockWriteGuard<()>> {
        let StepJobStatus::Started(locks) = &mut self.status else {
            eyre::bail!("cannot apply diff for a job that has not started");
        };
        locks.release_command_guard();
        Ok(ctx.hook_ctx.diff_lock.clone().write_owned().await)
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

    fn takes_write_locks(&self) -> bool {
        self.requested_run_type == RunType::Fix
            && !self.diffs_under_read_locks()
            && !self.rechecks_under_read_locks()
    }

    /// Whether this job is rerunning a read-only `check` after applying a
    /// patch (`check_after_diff`), which only needs read locks.
    fn rechecks_under_read_locks(&self) -> bool {
        self.rechecking_after_diff
            && self.run_type == RunType::Check
            && self
                .step
                .check
                .as_ref()
                .is_some_and(|check| check.effect() == Some(CommandEffect::Read))
    }

    fn try_flocks(&self, ctx: &StepContext) -> Option<Flocks> {
        if self.step.stomp {
            Some(Default::default())
        } else if self.takes_write_locks() {
            ctx.hook_ctx.file_locks.try_write(&self.files)
        } else {
            ctx.hook_ctx.file_locks.try_read(&self.files)
        }
    }

    async fn flocks(&self, ctx: &StepContext) -> Flocks {
        if self.step.stomp {
            Default::default()
        } else if self.takes_write_locks() {
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
            requested_run_type: self.requested_run_type,
            check_first: self.check_first,
            skip_reason: self.skip_reason.clone(),
            workspace_indicator: self.workspace_indicator.clone(),
            status: StepJobStatus::Pending,
            progress: self.progress.clone(),
            semaphore: None,
            diff_needs_write_locks: self.diff_needs_write_locks,
            rechecking_after_diff: self.rechecking_after_diff,
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
        let expected_files = step.shell_type().quote("src/main.ts");
        let job = StepJob::new(step, vec![PathBuf::from("ui/src/main.ts")], RunType::Check)
            .with_workspace_indicator(PathBuf::from("ui/tsconfig.json"));

        let tctx = job.tctx(&tera::Context::default());

        assert_eq!(tera::render("{{workspace}}", &tctx).unwrap(), ".");
        assert_eq!(
            tera::render("{{workspace_indicator}}", &tctx).unwrap(),
            "tsconfig.json"
        );
        assert_eq!(tera::render("{{files}}", &tctx).unwrap(), expected_files);
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

#[cfg(test)]
mod lock_mode_tests {
    use super::*;

    fn step_with_check(check: serde_json::Value) -> Arc<Step> {
        Arc::new(Step {
            check: Some(serde_json::from_value(check).unwrap()),
            check_diff: Some(
                serde_json::from_value(serde_json::json!({"command": "diff", "effect": "read"}))
                    .unwrap(),
            ),
            check_after_diff: true,
            ..Default::default()
        })
    }

    fn recheck_job(step: Arc<Step>) -> StepJob {
        let mut job = StepJob::new(step, vec![], RunType::Fix);
        job.run_type = RunType::Check;
        job.check_first = false;
        job.rechecking_after_diff = true;
        job
    }

    #[test]
    fn a_read_only_recheck_takes_read_locks() {
        let step = step_with_check(serde_json::json!({"command": "check", "effect": "read"}));
        assert!(!recheck_job(step).takes_write_locks());
    }

    #[test]
    fn a_recheck_that_may_write_keeps_write_locks() {
        let step = step_with_check(serde_json::json!({"command": "check", "effect": "write"}));
        assert!(recheck_job(step).takes_write_locks());
    }
}
