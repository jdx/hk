//! Step execution orchestration.
//!
//! This module contains `run_all_jobs`, the main entry point for executing a step.
//! It handles:
//!
//! - Waiting for dependencies
//! - Creating and spawning jobs concurrently
//! - Check-first mode with diff application
//! - File staging after fixes
//! - Progress tracking and error aggregation

use crate::error::Error;
use crate::git::Git;
use crate::hook::SkipReason;
use crate::step_context::StepContext;
use crate::step_job::StepJobStatus;
use crate::{Result, glob, tera};
use indexmap::IndexSet;
use itertools::Itertools;
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::OwnedSemaphorePermit;
use tokio_util::sync::CancellationToken;

use super::expr_env::eval_condition;
use super::types::{AllowFailure, CheckFirstCmd, RunType, Step};

/// How long jobs get to wind down after a cancellation (a fail-fast abort or a
/// user's Ctrl-C) before they are aborted. Cancelled commands are killed at once, so this only bounds a
/// job stuck somewhere unexpected.
const CANCELLED_JOBS_GRACE: Duration = Duration::from_secs(10);

/// Wait for every job task and return their results, or the most severe error.
///
/// A failed job does not end the wait. Dropping the set instead would abort
/// sibling jobs wherever they happen to be, including after their command
/// finished but before they recorded its diagnostics, so which diagnostics a
/// failed run reported depended on timing.
///
/// `on_failure` runs for each error until it returns a future. That future
/// cancels the siblings' commands (fail-fast); the siblings then get `grace`
/// to return before they are aborted. Without one (`--no-fail-fast`, an
/// allowed failure) siblings run to completion.
///
/// Whenever `cancel` is cancelled, including by a user's Ctrl-C while no job
/// has failed, the jobs get the same `grace` before they are aborted, so a
/// job stuck where the token is not observed cannot keep the run waiting.
/// Jobs aborted this way are not errors.
///
/// The error returned is the first one, except that an error `is_allowed`
/// rejects replaces an allowed one, so a job that could not run is never
/// hidden behind another job's allowed command failure. Other later errors,
/// such as a sibling reporting its command was cancelled, are only logged.
async fn join_jobs<T, Fut>(
    mut set: tokio::task::JoinSet<Result<T>>,
    cancel: &CancellationToken,
    grace: Duration,
    is_allowed: impl Fn(&eyre::Report) -> bool,
    mut on_failure: impl FnMut(&eyre::Report) -> Option<Fut>,
) -> Result<Vec<T>>
where
    T: 'static,
    Fut: Future<Output = ()>,
{
    let mut done = Vec::new();
    let mut first_error: Option<(eyre::Report, bool)> = None;
    let mut cancelled = false;
    let mut deadline = None;
    let mut cancel_seen = false;
    loop {
        let next = tokio::select! {
            biased;
            _ = async { tokio::time::sleep_until(deadline.unwrap()).await }, if deadline.is_some() => {
                debug!(
                    "jobs did not stop within the grace period after cancellation, aborting them"
                );
                set.abort_all();
                deadline = None;
                continue;
            }
            _ = cancel.cancelled(), if !cancel_seen => {
                cancel_seen = true;
                deadline.get_or_insert(tokio::time::Instant::now() + grace);
                continue;
            }
            next = set.join_next() => next,
        };
        let Some(res) = next else { break };
        let err = match res {
            Ok(Ok(value)) => {
                done.push(value);
                continue;
            }
            Ok(Err(err)) => err,
            Err(e) => match e.try_into_panic() {
                Ok(panic) => std::panic::resume_unwind(panic),
                Err(e) if e.is_cancelled() => {
                    debug!("job aborted after cancellation");
                    continue;
                }
                Err(e) => e.into(),
            },
        };
        if !cancelled && let Some(cancel) = on_failure(&err) {
            cancel.await;
            cancelled = true;
            deadline.get_or_insert(tokio::time::Instant::now() + grace);
        }
        let allowed = is_allowed(&err);
        match &first_error {
            None => first_error = Some((err, allowed)),
            Some((_, true)) if !allowed => first_error = Some((err, allowed)),
            Some(_) => debug!("another job failed after the first: {err:#}"),
        }
    }
    match first_error {
        Some((err, _)) => Err(err),
        None => Ok(done),
    }
}

/// Default stage pattern for steps with fix commands when staging is enabled.
static DEFAULT_STAGE: LazyLock<Vec<String>> = LazyLock::new(|| vec!["<JOB_FILES>".to_string()]);

impl Step {
    pub(crate) fn failure_is_allowed(&self, ctx: &expr::Context) -> Result<bool> {
        match &self.allow_failure {
            AllowFailure::Bool(allow) => Ok(*allow),
            AllowFailure::Expression(expression) => {
                let value = eval_condition(expression, ctx)?;
                match value {
                    expr::Value::Bool(allow) => Ok(allow),
                    _ => eyre::bail!("{self}: allow_failure expression must evaluate to a boolean"),
                }
            }
        }
    }

    /// Execute all jobs for this step.
    ///
    /// This is the main orchestration function that:
    /// 1. Waits for dependent steps to complete
    /// 2. Creates jobs based on files and configuration
    /// 3. Spawns jobs concurrently using tokio tasks
    /// 4. Handles check-first mode and diff application
    /// 5. Stages modified files after fixes
    /// 6. Updates progress tracking
    ///
    /// # Arguments
    ///
    /// * `ctx` - The step context (wrapped in Arc for sharing)
    /// * `semaphore` - Optional semaphore permit for concurrency control
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err` if any job fails
    pub(crate) async fn run_all_jobs(
        self: Arc<Self>,
        ctx: Arc<StepContext>,
        semaphore: Option<OwnedSemaphorePermit>,
        fail_fast: bool,
    ) -> Result<()> {
        let semaphore = self.wait_for_depends(&ctx, semaphore).await?;
        let ctx = Arc::new(ctx);

        if let Some(step_condition) = &self.step_condition {
            let val = eval_condition(step_condition, &ctx.hook_ctx.expr_ctx())?;
            debug!("{self}: condition: {step_condition} = {val}");
            if val == expr::Value::Bool(false) {
                self.mark_skipped(&ctx, &SkipReason::ConditionFalse)?;
                ctx.hook_ctx.dec_total_jobs(1);
                return Ok(());
            }
        }

        let files = ctx.hook_ctx.files();
        let mut jobs = self.build_step_jobs_shared(
            &files,
            ctx.hook_ctx.run_type,
            &ctx.hook_ctx.files_in_contention.lock().unwrap(),
            &ctx.hook_ctx.skip_steps,
            self.batch
                .then(|| {
                    ctx.batch_jobs.for_step(
                        &self.name,
                        &files,
                        ctx.hook_ctx.run_type,
                        &ctx.hook_ctx.skip_steps,
                    )
                })
                .flatten(),
        )?;
        // When this hook stages fixes with the default `stage`, a step that can
        // list or diff the files it would change checks first if any of its
        // files has unstaged changes, so only files it changed are fixed and
        // staged. Otherwise staging picks up exactly the files the fix changed,
        // and the check would only run the tool twice.
        if ctx.hook_ctx.should_stage
            && self.stage.is_none()
            && matches!(ctx.hook_ctx.run_type, RunType::Fix)
            && (self.check_list_files.is_some() || self.check_diff.is_some())
        {
            let unstaged = ctx.hook_ctx.initial_unstaged.lock().unwrap();
            for job in &mut jobs {
                if job.files.iter().any(|f| unstaged.contains(f)) {
                    job.check_first = true;
                }
            }
        }
        // A step that runs its fixer instead of applying its diff still checks
        // first when the hook stages fixes: in a typical commit the check
        // passes, which skips both the fixer and staging. This needs a
        // `check_diff` command for this platform; a platform-specific script
        // can be empty.
        if ctx.hook_ctx.should_stage
            && matches!(ctx.hook_ctx.run_type, RunType::Fix)
            && !self.applies_check_diff()
            && matches!(self.check_first_cmd(), Some(CheckFirstCmd::Diff(_)))
        {
            for job in &mut jobs {
                job.check_first = true;
            }
        }
        // Apply ARG_MAX-safe auto-batching now that the full tera context is
        // available — only split jobs whose rendered run command would actually
        // exceed the limit.
        let mut jobs = self.auto_batch_jobs(jobs, &ctx.hook_ctx.tctx)?;
        if let Some(job) = jobs.first_mut() {
            job.semaphore = Some(semaphore);
        }
        // Count all jobs (including those that will be marked skipped) for totals.
        // This avoids total being less than the number of completions we emit.
        let total_jobs_for_step = jobs.len();
        let non_skip_jobs = jobs.iter().filter(|j| j.skip_reason.is_none()).count();
        ctx.set_jobs_total(non_skip_jobs);
        if total_jobs_for_step > 0 {
            // Replace the single-step placeholder with the actual number of jobs.
            // Add the extra jobs beyond the placeholder 1.
            ctx.hook_ctx
                .inc_total_jobs(total_jobs_for_step.saturating_sub(1));
        } else {
            // If there are zero jobs after expansion, decrement the placeholder 1 we pre-added
            // for the step so the total does not exceed the number of completions.
            ctx.hook_ctx.dec_total_jobs(1);
            // Steps that `depends` on this one are waiting for it to finish; with
            // no jobs, nothing else would ever mark it done.
            self.mark_skipped(&ctx, &SkipReason::NoFilesToProcess)?;
            return Ok(());
        }
        // Capture the full set of files this step will actually operate on across all jobs.
        // We'll use this to scope staging so that broad stage globs (e.g., prettier's *.yaml)
        // cannot rope unrelated, non-job files into the index.
        let all_job_files: IndexSet<PathBuf> =
            jobs.iter().flat_map(|j| j.files.iter().cloned()).collect();

        let mut set = tokio::task::JoinSet::new();
        for job in jobs {
            let ctx = ctx.clone();
            let step = job.step.clone();
            let mut job = job;
            set.spawn(async move {
                let original_job_files = job.files.clone();
                // A check that keeps seeing new destinations or intervening
                // writes must eventually reach the fixer.
                const MAX_PATCH_RETRIES: usize = 8;
                let mut retried_existing_creations = HashSet::new();
                let mut patch_retries = 0;
                let mut focused_check_failed = false;
                let mut focused_check_output: Option<(String, String, String)> = None;
                if let Some(reason) = &job.skip_reason {
                    step.mark_skipped(&ctx, reason)?;
                    // Skipped jobs reduce the total rather than incrementing completed
                    // This shows actual work remaining vs work done
                    ctx.hook_ctx.dec_total_jobs(1);
                    return Ok(vec![]);
                }
                if job.check_first {
                    let prev_run_type = job.run_type;
                    job.run_type = RunType::Check;
                    let check_first_cmd = step.check_first_cmd();
                    // A patch computed under read locks goes stale if another step
                    // writes its files before this one can lock them for writing;
                    // `check_diff` then runs again, under write locks.
                    'check_first: loop {
                        match step.run(&ctx, &mut job).await {
                            Ok(()) => {
                                debug!("{step}: successfully ran check step first");
                                ctx.hook_ctx.inc_completed_jobs(1);
                                // When check and fix are the same command (a
                                // pre-commit-style fixer), this run was the fix: its
                                // files go to staging like any fixer's.
                                if step.check_is_fix() && !matches!(job.status, StepJobStatus::Pending)
                                {
                                    return Ok(job.files.clone());
                                }
                                return Ok(vec![]);
                            }
                            Err(e) => {
                                if let Some(Error::CheckListFailed { source: _, stdout, stderr, combined }) =
                                    e.downcast_ref::<Error>()
                                {
                                    debug!("{step}: failed check step first: check list or diff failed");
                                    // Log stderr if present (informational/warnings only)
                                    if !stderr.trim().is_empty() {
                                        debug!("{step}: check stderr output:\n{}", stderr);
                                    }
                                    // The command runner records ordinary diagnostic output, but
                                    // check-first errors return through a dedicated error type.
                                    // Preserve that listing/diff output for structured reporting.
                                    let diagnostic_dir =
                                        step.render_dir(&job.tctx(&ctx.hook_ctx.tctx))?;
                                    ctx.hook_ctx.append_diagnostic_output(
                                        &step.name,
                                        diagnostic_dir.as_deref(),
                                        combined,
                                    );
                                    if step.check_failed_files
                                        && matches!(prev_run_type, RunType::Check)
                                    {
                                        focused_check_failed = true;
                                        focused_check_output =
                                            Some((stdout.clone(), stderr.clone(), combined.clone()));
                                    }
                                    // Parse according to the check-first command that actually ran.
                                    // Platform-specific Script values can be empty, in which case
                                    // check_first_cmd falls back to the next available command.
                                    let (files, created, extras) = if matches!(
                                        check_first_cmd,
                                        Some(CheckFirstCmd::Diff(_))
                                    ) {
                                        let dir = step.render_dir(&job.tctx(&ctx.hook_ctx.tctx))?;
                                        let parsed = step.filter_files_from_check_diff(
                                            &job.files,
                                            stdout,
                                            dir.as_deref(),
                                        );
                                        // Patch application resolves `dir` before writing. Use
                                        // that same destination for locks and staging, including
                                        // when `dir` is a symlink into the repository.
                                        let root = std::env::current_dir()?.canonicalize()?;
                                        let base = PathBuf::from(dir.as_deref().unwrap_or("."));
                                        let base = base.canonicalize().unwrap_or(base);
                                        let created = parsed.created.into_iter().map(|path| {
                                            let path = if path.is_relative() {
                                                base.join(path)
                                            } else {
                                                path
                                            };
                                            path.strip_prefix(&root).unwrap_or(&path).to_path_buf()
                                        }).collect::<Vec<_>>();
                                        (parsed.files, created, parsed.extras)
                                    } else if matches!(
                                        check_first_cmd,
                                        Some(CheckFirstCmd::ListFiles(_))
                                    ) {
                                        let dir = step.render_dir(&job.tctx(&ctx.hook_ctx.tctx))?;
                                        let (files, extras) = step.filter_files_from_check_list(
                                            &job.files,
                                            stdout,
                                            dir.as_deref(),
                                        );
                                        (files, Vec::new(), extras)
                                    } else {
                                        (job.files.clone(), Vec::new(), Vec::new())
                                    };
                                    // Files the output names outside this job, which it
                                    // holds no locks on.
                                    let names_other_files = !extras.is_empty();
                                    for f in extras {
                                        warn!(
                                            "{step}: file in check output not found in original files: {}",
                                            f.display()
                                        );
                                    }

                                    // A creation header can name a file that was already in
                                    // this job. Keep it among the files sent to the fixer if
                                    // applying the patch fails or is disabled.
                                    let mut files = files;
                                    files.extend(
                                        created
                                            .iter()
                                            .filter(|path| original_job_files.contains(path))
                                            .cloned(),
                                    );
                                    let files: Vec<_> = files.into_iter().collect::<IndexSet<_>>().into_iter().collect();
                                    let patch_job_files = files.clone();
                                    // A creation-only patch still needs the original inputs if
                                    // application is disabled or falls back to the fixer.
                                    if files.is_empty()
                                        && matches!(check_first_cmd, Some(CheckFirstCmd::Diff(_)))
                                    {
                                        debug!("{step}: check_diff named no job files, keeping original files for the fixer");
                                    } else if files.is_empty()
                                        && matches!(check_first_cmd, Some(CheckFirstCmd::ListFiles(_)))
                                    {
                                        // For check_list_files: non-zero exit with no files is an error
                                        // (Tool failed, not "files need fixing")
                                        error!("{step}: check_list_files failed with no files in output");
                                        return Err(e);
                                    } else {
                                        job.files = files;
                                    }

                                    // Try to apply diff directly when check_diff is defined and we're in Fix mode
                                    // (prev_run_type is the original mode; job.run_type was temporarily changed to Check)
                                    // A step with `apply_check_diff = false` runs its fixer on the
                                    // files the diff names instead.
                                    if matches!(check_first_cmd, Some(CheckFirstCmd::Diff(_)))
                                        && prev_run_type == RunType::Fix
                                        && step.applies_check_diff()
                                    {
                                        let mut applied_files = if created.is_empty() {
                                            job.files.clone()
                                        } else {
                                            patch_job_files
                                        };
                                        applied_files.extend(created.iter().cloned());
                                        let fixer_files = job.files.clone();
                                        let newly_created: Vec<_> = created.iter().filter(|path| !original_job_files.contains(path)).cloned().collect();
                                        let under_read_locks = job.diffs_under_read_locks();
                                        // Stomp steps intentionally bypass file locks, so
                                        // there are no locks to upgrade or release here.
                                        if !step.stomp && (under_read_locks || !newly_created.is_empty()) {
                                            // Take every path needed by the patch in one call.
                                            // A write-effect check already holds its original
                                            // files, but must release them before waiting for a
                                            // newly created destination's lock.
                                            let mut named = if under_read_locks {
                                                applied_files.clone()
                                            } else {
                                                original_job_files.clone()
                                            };
                                            // The patch may depend on inputs it does not name.
                                            named.extend(original_job_files.iter().cloned());
                                            if !created.is_empty() {
                                                named.extend(created.iter().cloned());
                                            }
                                            let named: Vec<_> = named.into_iter().collect::<IndexSet<_>>().into_iter().collect();
                                            let locks = &ctx.hook_ctx.file_locks;
                                            let before = locks.write_counts(&named);
                                            job.files = named.clone();
                                            job.relock_for_write(&ctx).await?;
                                            let after = locks.write_counts(&named);
                                            let original_locked: HashSet<_> = original_job_files.iter().collect();
                                            let files_written_meanwhile = named.iter().zip(before.iter().zip(&after)).any(|(path, (&before, &after))| {
                                                // Releasing our own write locks increments their
                                                // counts once; an additional increment means
                                                // another writer ran while we waited.
                                                let own_release = u64::from(!under_read_locks && original_locked.contains(path));
                                                after != before.saturating_add(own_release)
                                            });
                                            let existing_creations: Vec<_> = newly_created.iter()
                                                .filter(|path| path.symlink_metadata().is_ok() && !retried_existing_creations.contains(*path))
                                                .cloned().collect();
                                            let retry_creation = !existing_creations.is_empty()
                                                && retried_existing_creations.len() < MAX_PATCH_RETRIES;
                                            if (under_read_locks && names_other_files) || files_written_meanwhile || retry_creation {
                                                if patch_retries == MAX_PATCH_RETRIES {
                                                    debug!("{step}: patch did not settle after {MAX_PATCH_RETRIES} retries, falling back to fixer");
                                                    job.files = fixer_files;
                                                    break 'check_first;
                                                }
                                                patch_retries += 1;
                                                retried_existing_creations.extend(existing_creations);
                                                debug!("{step}: files written meanwhile, diffing again");
                                                // Diff the job's files again, not just the ones the
                                                // patch named: a tool may derive its patch from all
                                                // of them, as `go mod tidy -diff` does for go.sum.
                                                job.files = original_job_files.clone();
                                                job.relock_for_write(&ctx).await?;
                                                continue 'check_first;
                                            }
                                        }
                                        // Apply where the check_diff command ran.
                                        let dir = step.render_dir(&job.tctx(&ctx.hook_ctx.tctx))?;
                                        let applied = {
                                            let _diff_guard = job.lock_diff(&ctx).await?;
                                            step.apply_diff_output(stdout, dir.as_deref())
                                        };
                                        match applied {
                                            Ok(true) => {
                                                if step.check_after_diff {
                                                    debug!(
                                                        "{step}: diff applied successfully, rerunning check on original files"
                                                    );
                                                    job.files = original_job_files.clone();
                                                    job.run_type = RunType::Check;
                                                    job.check_first = false;
                                                    job.rechecking_after_diff = true;
                                                    step.run(&ctx, &mut job).await?;
                                                } else {
                                                    debug!(
                                                        "{step}: diff applied successfully, skipping fixer"
                                                    );
                                                }
                                                ctx.hook_ctx.inc_completed_jobs(1);
                                                return Ok(applied_files);
                                            }
                                            Ok(false) => {
                                                // Diff application failed - fall through to run fixer
                                                debug!("{step}: diff application failed, falling back to fixer");
                                            }
                                            // Applying failed and the files couldn't all be
                                            // put back, so the fixer would start from damaged
                                            // files: stop instead.
                                            Err(err) => return Err(err),
                                        }
                                        // Lock acquisition may temporarily widen `job.files`.
                                        // The fixer receives only the files selected by the check.
                                        job.files = fixer_files;
                                    }
                                }
                                // For regular check commands that fail: fall through to run fixer
                                debug!("{step}: failed check step first: {e}");
                            }
                        }
                        break;
                    }
                    job.run_type = prev_run_type;
                    job.check_first = false;
                }
                // The initial auto-batching pass sizes the file-listing
                // command. Reapply it after narrowing so a larger focused
                // check command receives the same ARG_MAX protection.
                let jobs = if focused_check_failed {
                    let batch_error_job = job.clone();
                    match step.auto_batch_jobs(vec![job], &ctx.hook_ctx.tctx) {
                        Ok(jobs) => jobs,
                        Err(err) => {
                            if let Some((stdout, stderr, combined)) = &focused_check_output {
                                step.save_output_summary(
                                    &ctx,
                                    &batch_error_job,
                                    stdout,
                                    stderr,
                                    combined,
                                    true,
                                );
                            }
                            return Err(err);
                        }
                    }
                } else {
                    vec![job]
                };

                let mut files_to_return = IndexSet::new();
                let mut last_job = None;
                for (index, mut job) in jobs.into_iter().enumerate() {
                    // Focused batches run sequentially. Register each
                    // additional batch only when it is about to run so a
                    // failure cannot leave later, unrun batches in progress
                    // totals.
                    if index > 0 {
                        ctx.increment_job_count(1);
                        ctx.hook_ctx.inc_total_jobs(1);
                    }
                    let result = step.run(&ctx, &mut job).await;
                    if let Err(err) = &result {
                        if focused_check_failed
                            && let Some((stdout, stderr, combined)) = &focused_check_output
                        {
                            step.save_output_summary(
                                &ctx, &job, stdout, stderr, combined, true,
                            );
                        }
                        job.status_errored(&ctx, format!("{err}")).await?;
                    }
                    ctx.hook_ctx.inc_completed_jobs(1);
                    if !matches!(job.status, StepJobStatus::Pending) {
                        files_to_return.extend(job.files.clone());
                    }
                    result?;
                    last_job = Some(job);
                }

                // The file-listing check is authoritative. If every focused
                // diagnostic command completed successfully, keep the overall
                // step failed and preserve the original output. Cancellation
                // returns Ok without executing a command and must not be
                // reported as a contradictory success.
                if focused_check_failed && !ctx.hook_ctx.failed.is_cancelled() {
                    if let Some((stdout, stderr, combined)) = &focused_check_output
                        && let Some(job) = &last_job
                    {
                        step.save_output_summary(&ctx, job, stdout, stderr, combined, true);
                    }
                    let err = Error::FocusedCheckMismatch {
                        step: step.to_string(),
                    };
                    if let Some(job) = &mut last_job {
                        job.status_errored(&ctx, format!("{err}")).await?;
                    }
                    return Err(err.into());
                }
                Ok(files_to_return.into_iter().collect())
            });
        }
        // Every job is awaited, even after one fails, so a job that already
        // ran its check still records its diagnostics. See `join_jobs`.
        let failure_allowed = |err: &eyre::Report| {
            crate::error::is_command_failure(err)
                && self
                    .failure_is_allowed(&ctx.hook_ctx.expr_ctx())
                    .unwrap_or(false)
        };
        let job_files = join_jobs(
            set,
            &ctx.hook_ctx.failed,
            CANCELLED_JOBS_GRACE,
            failure_allowed,
            |err| {
                ctx.status_errored(&format!("{err}"));
                // A user's Ctrl-C already cancelled everything; it is not a step
                // failure to abort the siblings for.
                (fail_fast
                    && !failure_allowed(err)
                    && !crate::step_group::cancelled_by_user(&ctx.hook_ctx))
                .then(|| crate::step_group::abort_running_steps(&ctx.hook_ctx))
            },
        )
        .await?;
        let actual_job_files: IndexSet<PathBuf> = job_files.into_iter().flatten().collect();
        if ctx.hook_ctx.failed.is_cancelled() {
            ctx.status_aborted();
            return Ok(());
        }
        // Skip staging if no jobs actually processed any files (e.g., all jobs skipped by condition)
        if non_skip_jobs > 0
            && !actual_job_files.is_empty()
            && matches!(ctx.hook_ctx.run_type, RunType::Fix)
        {
            self.stage_files(&ctx, &all_job_files, &actual_job_files)
                .await?;
        }
        if non_skip_jobs > 0 {
            ctx.status_finished();
            ctx.depends.mark_done(&self.name)?;
        }
        Ok(())
    }

    /// Wait for dependent steps to complete before running this step.
    ///
    /// Releases the semaphore while waiting so other steps can run.
    async fn wait_for_depends(
        &self,
        ctx: &StepContext,
        mut semaphore: Option<OwnedSemaphorePermit>,
    ) -> Result<OwnedSemaphorePermit> {
        for dep in &self.depends {
            if !ctx.depends.is_done(dep) {
                debug!("{self}: waiting for {dep}");
                semaphore.take(); // release semaphore for another step
            }
            ctx.depends.wait_for(dep).await?;
        }
        match semaphore {
            Some(semaphore) => Ok(semaphore),
            None => Ok(ctx.hook_ctx.semaphore().await),
        }
    }

    /// Stage modified files after running fix commands.
    ///
    /// This handles the complex logic of determining which files to stage:
    /// - Respects the `stage` configuration patterns
    /// - Scopes staging to files actually processed by this step
    /// - Handles `<JOB_FILES>` special value
    /// - Shares one `git add` with steps that are staging at the same time
    async fn stage_files(
        &self,
        ctx: &StepContext,
        all_job_files: &IndexSet<PathBuf>,
        actual_job_files: &IndexSet<PathBuf>,
    ) -> Result<()> {
        // Build stage pathspecs; if `dir` is set, stage entries are relative to it.
        // Compute "root" variants for patterns that start with "**/" BEFORE prefixing with `dir`.
        // A step-level stage setting filters paths; it never enables staging.
        // When staging is enabled, explicit patterns win and fixers otherwise
        // default to the files processed by this job.
        let effective_stage: Option<&Vec<String>> = if !ctx.hook_ctx.should_stage {
            None
        } else if self.stage.is_some() {
            self.stage.as_ref()
        } else if self.fix.is_some() || self.check_diff.is_some() {
            Some(&DEFAULT_STAGE)
        } else {
            None
        };

        // Special case: if stage is exactly "<JOB_FILES>", use actual_job_files directly
        let stage_only_job_files = effective_stage
            .map(|v| v.len() == 1 && v[0] == "<JOB_FILES>")
            .unwrap_or(false);

        let rendered_patterns: Vec<String> = if stage_only_job_files {
            // Don't render the template, we'll use actual_job_files directly
            vec![]
        } else {
            effective_stage
                .unwrap_or(&vec![])
                .iter()
                .map(|s| tera::render(s, &ctx.hook_ctx.tctx))
                .collect::<Result<Vec<_>>>()?
        };

        // One root per directory the step's jobs ran in: a templated `dir` needs
        // a pattern per workspace, or `generated/**` would resolve at the repo
        // root and both miss the per-workspace files and match unrelated ones.
        let stage_roots = self.resolved_dirs(&ctx.hook_ctx.tctx, all_job_files)?;
        if !rendered_patterns.is_empty() && self.dir.is_some() && stage_roots.is_empty() {
            warn!(
                "{self}: `stage` patterns are relative to the repo root: `dir` is templated and no workspace matched, so hk cannot scope them"
            );
        }

        let mut stage_globs: Vec<String> = Vec::new();
        for pat in rendered_patterns {
            // Always include the base pattern (under each dir if present)
            push_stage_globs(&mut stage_globs, &stage_roots, &pat);

            // If the original (un-prefixed) pattern starts with "**/", also include a root-level variant
            // without that prefix. When `dir` is set, make the root variant relative to `dir`.
            if let Some(rest) = pat.strip_prefix("**/")
                && !rest.is_empty()
            {
                push_stage_globs(&mut stage_globs, &stage_roots, rest);
            }
        }
        // Guard against empty pathspecs (e.g., when pattern is exactly "**/")
        stage_globs.retain(|g| !g.is_empty());
        // Ignore directory-only patterns (ending with '/'); staging should target files
        stage_globs.retain(|g| !g.ends_with('/'));
        trace!("{}: stage globs: {:?}", self, stage_globs);
        let stage_pathspecs: Vec<OsString> =
            stage_globs.iter().cloned().map(OsString::from).collect();
        if !stage_pathspecs.is_empty() || stage_only_job_files {
            // Other steps may still be fixing files that `status` hashes and `add`
            // reads. Hold read locks on everything the status query can inspect
            // until the add finishes, so git never reads a partially written file
            // (both libgit2 and the git CLI fail when a file changes mid-read).
            // Take the file locks before the git mutex so neither waits on the other.
            // The status query inspects only the files it is asked about, so
            // staging does not wait for steps that write other files.
            let status_files = if stage_only_job_files {
                actual_job_files.iter().cloned().collect_vec()
            } else {
                glob::get_matches(&stage_globs, &ctx.hook_ctx.files())?
            };
            // `git add` writes the index, which re-hashes racily clean entries
            // anywhere in the repository (see `Git::racily_clean_paths`), so it
            // also needs read locks on those. Take every lock in one call, since
            // waiting for file locks while holding others could deadlock, then
            // check under the git mutex that no entry became racy meanwhile.
            // Nothing else writes the index while the mutex is held.
            let mut lock_files: BTreeSet<PathBuf> = status_files.iter().cloned().collect();
            if ctx.hook_ctx.should_stage {
                let git = ctx.hook_ctx.git.lock().await;
                lock_files.extend(racy_hook_files(ctx, &git, &lock_files));
            }
            let (_flocks, _diff_guard, git) = loop {
                let lock_vec = lock_files.iter().cloned().collect_vec();
                let flocks = ctx.hook_ctx.file_locks.read_locks(&lock_vec).await;
                // Patches can write outside their job's declared inputs. Keep
                // shared access through status and the coalesced staging queue.
                // Drop all guards before retrying with additional file locks.
                let diff_guard = ctx.hook_ctx.diff_lock.read().await;
                let git = ctx.hook_ctx.git.lock().await;
                if !ctx.hook_ctx.should_stage {
                    break (flocks, diff_guard, git);
                }
                let racy = racy_hook_files(ctx, &git, &lock_files);
                if racy.is_empty() {
                    break (flocks, diff_guard, git);
                }
                trace!("{self}: more racily clean files to lock: {racy:?}");
                lock_files.extend(racy);
            };
            let status = if stage_only_job_files {
                // Only this job's files can be staged, so only they need a status
                git.status_of_paths(&status_files)?
            } else {
                git.status_of_pathspec(&stage_pathspecs)?
            };

            // Build a scoped candidate set:
            //  - Include files that this step actually operated on (union of job files)
            //  - Include explicit, non-glob stage paths (to allow generators)
            //  - Include files from status that match the stage globs (untracked/unstaged)
            //    since status was filtered by stage_pathspecs
            let is_globlike = |s: &str| s.contains('*') || s.contains('?') || s.contains('[');
            let mut candidates: IndexSet<PathBuf> = if stage_only_job_files {
                // When stage=<JOB_FILES>, use the actual files processed (after check_list_files filtering)
                trace!(
                    "{}: using actual_job_files for stage candidates: {:?}",
                    self, actual_job_files
                );
                actual_job_files.clone()
            } else {
                // Default behavior: start with all files matched by glob
                all_job_files.clone()
            };

            if !stage_only_job_files {
                for pat in &stage_globs {
                    if !is_globlike(pat) {
                        let p = PathBuf::from(pat);
                        if p.exists() {
                            candidates.insert(p);
                        }
                    }
                }

                // status was filtered by stage_pathspecs, so these files already match the globs
                for p in status.untracked_files.iter() {
                    candidates.insert(p.clone());
                }
                for p in status.unstaged_files.iter() {
                    candidates.insert(p.clone());
                }
            }
            // else: when stage=<JOB_FILES>, candidates only contains actual_job_files

            let candidate_vec = candidates.into_iter().collect_vec();
            let matched_candidates = if stage_only_job_files {
                // For <JOB_FILES>, all candidates are already the files we want
                candidate_vec
            } else {
                glob::get_matches(&stage_globs, &candidate_vec)?
            };

            // Now keep only those that are actually unstaged or untracked.
            // When using the default stage=<JOB_FILES>, exclude files that were already
            // untracked before the hook started — only stage untracked files that were
            // newly created by a fixer. Explicit stage globs opt into staging all
            // matching untracked files.
            let unstaged_set: IndexSet<PathBuf> = status.unstaged_files.iter().cloned().collect();
            let untracked_set: IndexSet<PathBuf> = status.untracked_files.iter().cloned().collect();
            let mut filtered = matched_candidates
                .into_iter()
                .filter(|p| {
                    if untracked_set.contains(p) {
                        if stage_only_job_files {
                            // Only stage untracked files that are newly created (not pre-existing)
                            !ctx.hook_ctx.initial_untracked.contains(p)
                        } else {
                            true
                        }
                    } else {
                        unstaged_set.contains(p)
                    }
                })
                .collect_vec();

            trace!(
                "{}: files to stage after filtering/scoping: {:?}",
                self, filtered
            );
            // Only stage matched files when staging is enabled for this hook.
            // Unintended staging caused by stash/apply is handled separately in git.pop_stash().
            if ctx.hook_ctx.should_stage && !filtered.is_empty() {
                // Share one `git add` with the steps staging at the same time.
                // Files one of them already queued are left out of `filtered`:
                // had its `git add` run first, this status would not list them.
                // Every step with queued files keeps its read locks until the
                // queue is staged, and nothing writes the index until then, so
                // the racily clean entries each step checked above are
                // unchanged and locked.
                let hook_ctx = &ctx.hook_ctx;
                hook_ctx
                    .stage_queue
                    .stage(&hook_ctx.git, git, &mut filtered, |git, paths| {
                        git.add(paths)?;
                        hook_ctx.add_files(paths, &[]);
                        Ok(())
                    })
                    .await?;
            }
            if !filtered.is_empty() {
                // Snapshot pre-staging untracked set for classification
                let pre_untracked: BTreeSet<PathBuf> = status.untracked_files.clone();
                // Classify staged files using pre-staging untracked snapshot
                let filtered_set: BTreeSet<PathBuf> = filtered.iter().cloned().collect();
                let created_paths: BTreeSet<PathBuf> =
                    filtered_set.intersection(&pre_untracked).cloned().collect();
                let added_paths: BTreeSet<PathBuf> =
                    filtered_set.difference(&created_paths).cloned().collect();
                let added_paths: Vec<PathBuf> = added_paths.iter().cloned().collect();
                let created_paths: Vec<PathBuf> = created_paths.iter().cloned().collect();
                ctx.add_files(&added_paths, &created_paths);
            }
        }
        Ok(())
    }
}

/// Hook files that an index write may read and that are not in `locked`.
/// If the index can't be read, every hook file counts.
fn racy_hook_files(ctx: &StepContext, git: &Git, locked: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    let hook_files = ctx.hook_ctx.files();
    let racy = match git.racily_clean_paths() {
        Ok(racy) => racy,
        Err(err) => {
            debug!("failed to find racily clean index entries, locking all files: {err:?}");
            return hook_files
                .into_iter()
                .filter(|p| !locked.contains(p))
                .collect();
        }
    };
    let hook_files: HashSet<PathBuf> = hook_files.into_iter().collect();
    racy.into_iter()
        .filter(|p| hook_files.contains(p) && !locked.contains(p))
        .collect()
}

/// Push `pat` once per stage root, or bare when there are none (the repo root).
fn push_stage_globs(globs: &mut Vec<String>, roots: &[String], pat: &str) {
    if roots.is_empty() {
        globs.push(pat.to_string());
        return;
    }
    for root in roots {
        let root = root.trim_end_matches('/');
        if root.is_empty() || root == "." {
            globs.push(pat.to_string());
        } else {
            globs.push(format!("{root}/{pat}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    const GRACE: Duration = Duration::from_secs(10);

    /// A job that records something after a delay, standing in for a job whose
    /// command has finished but which has not yet recorded its diagnostics.
    fn recorder(
        set: &mut tokio::task::JoinSet<Result<()>>,
        recorded: &Arc<Mutex<Vec<&'static str>>>,
        name: &'static str,
        delay: Duration,
    ) {
        let recorded = recorded.clone();
        set.spawn(async move {
            tokio::time::sleep(delay).await;
            recorded.lock().unwrap().push(name);
            Ok(())
        });
    }

    #[tokio::test]
    async fn join_jobs_waits_for_siblings_after_a_failure() {
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let mut set = tokio::task::JoinSet::<Result<()>>::new();
        set.spawn(async { Err(eyre::eyre!("first failure")) });
        recorder(&mut set, &recorded, "slow", Duration::from_millis(200));
        let result = join_jobs(
            set,
            &CancellationToken::new(),
            GRACE,
            |_| false,
            |_| None::<std::future::Ready<()>>,
        )
        .await;
        assert_eq!(result.unwrap_err().to_string(), "first failure");
        assert_eq!(*recorded.lock().unwrap(), ["slow"]);
    }

    #[tokio::test]
    async fn join_jobs_cancels_siblings_but_lets_them_finish() {
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let cancel = CancellationToken::new();
        let mut set = tokio::task::JoinSet::<Result<()>>::new();
        set.spawn(async { Err(eyre::eyre!("first failure")) });
        {
            // Stops waiting as soon as it is cancelled, then still records.
            let (recorded, cancel) = (recorded.clone(), cancel.clone());
            set.spawn(async move {
                tokio::select! {
                    _ = cancel.cancelled() => {}
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {}
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
                recorded.lock().unwrap().push("cancelled");
                Err(eyre::eyre!("cancelled"))
            });
        }
        let started = tokio::time::Instant::now();
        let result = join_jobs(
            set,
            &CancellationToken::new(),
            GRACE,
            |_| false,
            |_| {
                let cancel = cancel.clone();
                Some(async move { cancel.cancel() })
            },
        )
        .await;
        // The first error wins, the cancelled sibling's own error is dropped,
        // and the sibling was not waited for beyond its cancellation.
        assert_eq!(result.unwrap_err().to_string(), "first failure");
        assert_eq!(*recorded.lock().unwrap(), ["cancelled"]);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn join_jobs_aborts_siblings_that_ignore_cancellation() {
        let mut set = tokio::task::JoinSet::<Result<()>>::new();
        set.spawn(async { Err(eyre::eyre!("first failure")) });
        set.spawn(async {
            std::future::pending::<()>().await;
            Ok(())
        });
        let result = join_jobs(
            set,
            &CancellationToken::new(),
            Duration::from_millis(50),
            |_| false,
            |_| Some(async {}),
        )
        .await;
        assert_eq!(result.unwrap_err().to_string(), "first failure");
    }

    /// Errors named "allowed*" count as allowed command failures.
    async fn first_error(errors: &[&'static str]) -> String {
        let mut set = tokio::task::JoinSet::<Result<()>>::new();
        for (i, name) in errors.iter().enumerate() {
            let name = *name;
            set.spawn(async move {
                tokio::time::sleep(Duration::from_millis(50 * i as u64)).await;
                Err(eyre::eyre!(name))
            });
        }
        join_jobs(
            set,
            &CancellationToken::new(),
            GRACE,
            |e| e.to_string().starts_with("allowed"),
            |_| None::<std::future::Ready<()>>,
        )
        .await
        .unwrap_err()
        .to_string()
    }

    #[tokio::test]
    async fn join_jobs_hard_error_beats_earlier_allowed_failure() {
        assert_eq!(
            first_error(&["allowed one", "hard spawn error"]).await,
            "hard spawn error"
        );
    }

    #[tokio::test]
    async fn join_jobs_allowed_failure_does_not_replace_hard_error() {
        assert_eq!(
            first_error(&["hard spawn error", "allowed one"]).await,
            "hard spawn error"
        );
    }

    #[tokio::test]
    async fn join_jobs_first_of_equal_severity_wins() {
        assert_eq!(first_error(&["hard a", "hard b"]).await, "hard a");
        assert_eq!(first_error(&["allowed a", "allowed b"]).await, "allowed a");
    }

    #[tokio::test]
    async fn join_jobs_cancels_when_hard_error_follows_allowed_failure() {
        let cancel = CancellationToken::new();
        let mut set = tokio::task::JoinSet::<Result<()>>::new();
        set.spawn(async { Err(eyre::eyre!("allowed one")) });
        {
            let cancel = cancel.clone();
            set.spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Err(eyre::eyre!("hard spawn error"))
            });
            set.spawn(async move {
                cancel.cancelled().await;
                Ok(())
            });
        }
        let result = join_jobs(
            set,
            &CancellationToken::new(),
            GRACE,
            |e| e.to_string().starts_with("allowed"),
            |e| {
                let cancel = cancel.clone();
                (!e.to_string().starts_with("allowed")).then_some(async move { cancel.cancel() })
            },
        )
        .await;
        assert_eq!(result.unwrap_err().to_string(), "hard spawn error");
    }

    #[tokio::test]
    async fn join_jobs_returns_every_result_when_all_succeed() {
        let mut set = tokio::task::JoinSet::new();
        for n in 0..3 {
            set.spawn(async move { Ok(n) });
        }
        let mut done = join_jobs(
            set,
            &CancellationToken::new(),
            GRACE,
            |_| false,
            |_| None::<std::future::Ready<()>>,
        )
        .await
        .unwrap();
        done.sort();
        assert_eq!(done, [0, 1, 2]);
    }

    #[tokio::test]
    async fn join_jobs_bounds_the_wait_after_user_cancel_without_a_failure() {
        // No job fails; the token is cancelled from outside (Ctrl-C) and one
        // job ignores it. The wait ends within the grace and is not an error.
        let cancel = CancellationToken::new();
        let mut set = tokio::task::JoinSet::<Result<()>>::new();
        set.spawn(async { Ok(()) });
        set.spawn(async {
            std::future::pending::<()>().await;
            Ok(())
        });
        {
            let cancel = cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                cancel.cancel();
            });
        }
        let started = tokio::time::Instant::now();
        let done = join_jobs(
            set,
            &cancel,
            Duration::from_millis(200),
            |_| false,
            |_| None::<std::future::Ready<()>>,
        )
        .await
        .unwrap();
        assert_eq!(done.len(), 1);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[tokio::test]
    async fn join_jobs_bounds_the_wait_after_user_cancel_and_keeps_the_flag_clear() {
        // A user cancellation is not a failure to abort siblings for
        // (`on_failure` returns None), yet a stuck sibling is still aborted
        // within the grace and the first error is returned unchanged.
        let cancel = CancellationToken::new();
        let on_failure_called = std::sync::atomic::AtomicBool::new(false);
        let mut set = tokio::task::JoinSet::<Result<()>>::new();
        {
            let cancel = cancel.clone();
            set.spawn(async move {
                cancel.cancel();
                Err(eyre::eyre!("cancelled by user"))
            });
        }
        set.spawn(async {
            std::future::pending::<()>().await;
            Ok(())
        });
        let started = tokio::time::Instant::now();
        let result = join_jobs(
            set,
            &cancel,
            Duration::from_millis(200),
            |_| false,
            |_| {
                on_failure_called.store(true, std::sync::atomic::Ordering::SeqCst);
                None::<std::future::Ready<()>>
            },
        )
        .await;
        assert_eq!(result.unwrap_err().to_string(), "cancelled by user");
        assert!(started.elapsed() < Duration::from_secs(5));
        // The callback that would mark `fail_fast_aborted` ran but returned no
        // cancellation, so the flag it guards stays clear.
        assert!(on_failure_called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
