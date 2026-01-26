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
use crate::hook::SkipReason;
use crate::step_context::StepContext;
use crate::step_job::StepJobStatus;
use crate::{Result, glob, tera};
use indexmap::IndexSet;
use itertools::Itertools;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tokio::sync::OwnedSemaphorePermit;

use super::expr_env::EXPR_ENV;
use super::types::{RunType, Step};

/// Default stage pattern for steps with fix commands when staging is enabled.
static DEFAULT_STAGE: LazyLock<Vec<String>> = LazyLock::new(|| vec!["<JOB_FILES>".to_string()]);

impl Step {
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
        &self,
        ctx: Arc<StepContext>,
        semaphore: Option<OwnedSemaphorePermit>,
    ) -> Result<()> {
        let semaphore = self.wait_for_depends(&ctx, semaphore).await?;
        let ctx = Arc::new(ctx);

        if let Some(step_condition) = &self.step_condition {
            let val = EXPR_ENV.eval(&step_condition, &ctx.hook_ctx.expr_ctx())?;
            debug!("{self}: condition: {step_condition} = {val}");
            if val == expr::Value::Bool(false) {
                self.mark_skipped(&ctx, &SkipReason::ConditionFalse)?;
                return Ok(());
            }
        }

        let files = ctx.hook_ctx.files();
        let mut jobs = self.build_step_jobs(
            &files,
            ctx.hook_ctx.run_type,
            &ctx.hook_ctx.files_in_contention.lock().unwrap(),
            &ctx.hook_ctx.skip_steps,
        )?;
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
        }
        // Capture the full set of files this step will actually operate on across all jobs.
        // We'll use this to scope staging so that broad stage globs (e.g., prettier's *.yaml)
        // cannot rope unrelated, non-job files into the index.
        let all_job_files: IndexSet<PathBuf> = jobs.iter().flat_map(|j| j.files.clone()).collect();

        let mut set = tokio::task::JoinSet::new();
        for job in jobs {
            let ctx = ctx.clone();
            let step = self.clone();
            let mut job = job;
            set.spawn(async move {
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
                    match step.run(&ctx, &mut job).await {
                        Ok(()) => {
                            debug!("{step}: successfully ran check step first");
                            ctx.hook_ctx.inc_completed_jobs(1);
                            return Ok(vec![]);
                        }
                        Err(e) => {
                            if let Some(Error::CheckListFailed { source: _, stdout, stderr }) =
                                e.downcast_ref::<Error>()
                            {
                                debug!("{step}: failed check step first: check list or diff failed");
                                // Log stderr if present (informational/warnings only)
                                if !stderr.trim().is_empty() {
                                    debug!("{step}: check stderr output:\n{}", stderr);
                                }
                                // Use check_diff parser if check_diff is defined, otherwise check_list_files
                                let is_check_diff = step.check_diff.is_some();
                                let (files, extras) = if is_check_diff {
                                    step.filter_files_from_check_diff(&job.files, stdout)
                                } else {
                                    step.filter_files_from_check_list(&job.files, stdout)
                                };
                                for f in extras {
                                    warn!(
                                        "{step}: file in check output not found in original files: {}",
                                        f.display()
                                    );
                                }

                                // For check_diff: if no parseable files, keep all original files
                                if files.is_empty() && is_check_diff {
                                    debug!("{step}: check_diff returned no parseable files, will run fixer on all original files");
                                    // Keep all original files for check_diff when diff parsing fails
                                } else if files.is_empty() {
                                    // For check_list_files: non-zero exit with no files is an error
                                    // (Tool failed, not "files need fixing")
                                    error!("{step}: check_list_files failed with no files in output");
                                    return Err(e);
                                } else {
                                    job.files = files;
                                }

                                // Try to apply diff directly when check_diff is defined and we're in Fix mode
                                // (prev_run_type is the original mode; job.run_type was temporarily changed to Check)
                                if is_check_diff && prev_run_type == RunType::Fix {
                                    match step.apply_diff_output(stdout) {
                                        Ok(true) => {
                                            // Diff applied successfully - no need to run fixer
                                            debug!("{step}: diff applied successfully, skipping fixer");
                                            job.run_type = prev_run_type;
                                            ctx.hook_ctx.inc_completed_jobs(1);
                                            return Ok(job.files.clone());
                                        }
                                        Ok(false) => {
                                            // Diff application failed - fall through to run fixer
                                            debug!("{step}: diff application failed, falling back to fixer");
                                        }
                                        Err(err) => {
                                            // Unexpected error - fall through to run fixer
                                            warn!("{step}: unexpected error applying diff: {err}");
                                        }
                                    }
                                }
                            }
                            // For regular check commands that fail: fall through to run fixer
                            debug!("{step}: failed check step first: {e}");
                        }
                    }
                    job.run_type = prev_run_type;
                    job.check_first = false;
                }
                let result = step.run(&ctx, &mut job).await;
                if let Err(err) = &result {
                    job.status_errored(&ctx, format!("{err}")).await?;
                }
                ctx.hook_ctx.inc_completed_jobs(1);
                // Return the actual files that were processed after filtering
                // If job was skipped (status still Pending or marked skipped), return empty
                let files_to_return = if matches!(job.status, StepJobStatus::Pending) {
                    vec![]
                } else {
                    job.files
                };
                result.map(|_| files_to_return)
            });
        }
        let mut actual_job_files: IndexSet<PathBuf> = IndexSet::new();
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(files)) => {
                    actual_job_files.extend(files);
                }
                Ok(Err(err)) => {
                    ctx.status_errored(&format!("{err}"));
                    return Err(err);
                }
                Err(e) => match e.try_into_panic() {
                    Ok(e) => std::panic::resume_unwind(e),
                    Err(e) => {
                        ctx.status_errored(&format!("{e}"));
                        return Err(e.into());
                    }
                },
            }
        }
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
    async fn stage_files(
        &self,
        ctx: &StepContext,
        all_job_files: &IndexSet<PathBuf>,
        actual_job_files: &IndexSet<PathBuf>,
    ) -> Result<()> {
        // Build stage pathspecs; if `dir` is set, stage entries are relative to it
        // Compute "root" variants for patterns that start with "**/" BEFORE prefixing with `dir`.
        // Determine effective stage: explicit setting wins, otherwise default to <JOB_FILES>
        // for steps with fix commands when staging is enabled.
        let effective_stage: Option<&Vec<String>> = if self.stage.is_some() {
            self.stage.as_ref()
        } else if ctx.hook_ctx.should_stage && self.fix.is_some() {
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

        let mut stage_globs: Vec<String> = Vec::new();
        for pat in rendered_patterns {
            // Always include the base pattern (with dir prefix if present)
            if let Some(dir) = &self.dir {
                stage_globs.push(format!("{}/{}", dir.trim_end_matches('/'), pat));
            } else {
                stage_globs.push(pat.clone());
            }

            // If the original (un-prefixed) pattern starts with "**/", also include a root-level variant
            // without that prefix. When `dir` is set, make the root variant relative to `dir`.
            if let Some(rest) = pat.strip_prefix("**/")
                && !rest.is_empty()
            {
                if let Some(dir) = &self.dir {
                    stage_globs.push(format!("{}/{}", dir.trim_end_matches('/'), rest));
                } else {
                    stage_globs.push(rest.to_string());
                }
            }
        }
        // Guard against empty pathspecs (e.g., when pattern is exactly "**/")
        stage_globs.retain(|g| !g.is_empty());
        // Ignore directory-only patterns (ending with '/'); staging should target files
        stage_globs.retain(|g| !g.ends_with('/'));
        trace!("{}: stage globs: {:?}", self, &stage_globs);
        let stage_pathspecs: Vec<OsString> =
            stage_globs.iter().cloned().map(OsString::from).collect();
        if !stage_pathspecs.is_empty() || stage_only_job_files {
            let status = if stage_only_job_files {
                // For {{job_files}}, get status of all files (no pathspec filtering)
                ctx.hook_ctx.git.lock().await.status(None)?
            } else {
                ctx.hook_ctx
                    .git
                    .lock()
                    .await
                    .status(Some(&stage_pathspecs))?
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

            // Now keep only those that are actually unstaged or untracked
            let unstaged_set: IndexSet<PathBuf> = status.unstaged_files.iter().cloned().collect();
            let untracked_set: IndexSet<PathBuf> = status.untracked_files.iter().cloned().collect();
            let filtered = matched_candidates
                .into_iter()
                .filter(|p| unstaged_set.contains(p) || untracked_set.contains(p))
                .collect_vec();

            trace!(
                "{}: files to stage after filtering/scoping: {:?}",
                self, &filtered
            );
            if !filtered.is_empty() {
                // Snapshot pre-staging untracked set for classification
                let pre_untracked: BTreeSet<PathBuf> = status.untracked_files.clone();
                // Only stage matched files if stage setting is enabled (default: true)
                // Unintended staging caused by stash/apply is handled separately in git.pop_stash().
                if ctx.hook_ctx.should_stage {
                    ctx.hook_ctx.git.lock().await.add(&filtered)?;
                }
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
