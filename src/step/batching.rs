//! Job batching to prevent ARG_MAX overflow.
//!
//! When passing large file lists to shell commands, the total argument length can exceed
//! the operating system's `ARG_MAX` limit, causing "Argument list too long" errors.
//!
//! This module renders the actual run command for each job and only splits jobs whose
//! rendered command exceeds the safe limit. Steps whose commands don't reference
//! `{{files}}` (or render to a small string for any other reason) are left as a
//! single job, even when the underlying file list is large.

use crate::env;
use crate::step_job::StepJob;
use crate::tera;
use std::path::PathBuf;
use std::sync::Arc;

use super::types::Step;

impl Step {
    /// Estimates the size of the `{{files}}` template variable expansion.
    ///
    /// Used as a fallback when rendering the run command fails (e.g. the
    /// template references a variable not yet present in the context).
    pub(crate) fn estimate_files_string_size(&self, files: &[PathBuf]) -> usize {
        files
            .iter()
            .map(|f| {
                let path_str = f.to_str().unwrap_or("");
                // Worst-case quoted size: 2x + 2 (quotes), +1 for space separator
                path_str.len() * 2 + 2 + 1
            })
            .sum()
    }

    /// Render the run command for a hypothetical job containing `files` and return
    /// its byte length. Returns `None` if rendering fails (e.g. the template
    /// references a variable not in the context).
    fn render_run_command_size(
        &self,
        original_job: &StepJob,
        files: &[PathBuf],
        base_tctx: &tera::Context,
    ) -> Option<usize> {
        let run_cmd = if original_job.check_first {
            self.check_first_cmd()
        } else {
            self.run_cmd(original_job.run_type)
        }?;
        let run = run_cmd.to_string();
        if run.trim().is_empty() {
            return None;
        }
        let run = if let Some(prefix) = &self.prefix {
            format!("{prefix} {run}")
        } else {
            run
        };

        let mut temp = StepJob::new(
            Arc::clone(&original_job.step),
            files.to_vec(),
            original_job.run_type,
        );
        temp.check_first = original_job.check_first;
        if let Some(wi) = original_job.workspace_indicator() {
            temp = temp.with_workspace_indicator(wi.clone());
        }
        let tctx = temp.tctx(base_tctx);
        tera::render(&run, &tctx).ok().map(|s| s.len())
    }

    /// Automatically batch jobs whose rendered run command would exceed the safe ARG_MAX limit.
    ///
    /// Uses 50% of `ARG_MAX` as the safety margin (accounts for env vars and the command itself).
    /// Renders the actual run command with each candidate file subset; if the rendered command
    /// fits, no batching is performed. Otherwise binary-searches the largest batch size whose
    /// rendered command still fits.
    ///
    /// If rendering fails for any reason, falls back to estimating the size of the quoted
    /// file-list expansion — preserves the previous (purely size-based) behavior as a safety net.
    pub(crate) fn auto_batch_jobs(
        &self,
        jobs: Vec<StepJob>,
        base_tctx: &tera::Context,
    ) -> Vec<StepJob> {
        if self.stdin.is_some() {
            // stdin path doesn't pass files via argv; never auto-batch
            return jobs;
        }

        let safe_limit = *env::ARG_MAX / 2;
        let mut batched_jobs = Vec::with_capacity(jobs.len());

        for job in jobs {
            if job.skip_reason.is_some() || job.files.len() <= 1 {
                batched_jobs.push(job);
                continue;
            }

            // Try render-based sizing first; fall back to byte estimation on render failure.
            let full_size = self
                .render_run_command_size(&job, &job.files, base_tctx)
                .unwrap_or_else(|| self.estimate_files_string_size(&job.files));

            if full_size <= safe_limit {
                batched_jobs.push(job);
                continue;
            }

            debug!(
                "{}: auto-batching {} files (rendered size: {} bytes, limit: {} bytes)",
                self.name,
                job.files.len(),
                full_size,
                safe_limit
            );

            // Binary search the largest batch size whose rendered command fits.
            let mut low = 1;
            let mut high = job.files.len();
            while low < high {
                let mid = (low + high).div_ceil(2);
                let test_size = self
                    .render_run_command_size(&job, &job.files[..mid], base_tctx)
                    .unwrap_or_else(|| self.estimate_files_string_size(&job.files[..mid]));
                if test_size <= safe_limit {
                    low = mid;
                } else {
                    high = mid - 1;
                }
            }
            let batch_size = low.max(1);

            debug!(
                "{}: using batch size of {} files per batch",
                self.name, batch_size
            );

            for chunk in job.files.chunks(batch_size) {
                let mut new_job = StepJob::new(Arc::clone(&job.step), chunk.to_vec(), job.run_type);
                // Preserve job-level state that isn't reconstructed by StepJob::new.
                new_job.check_first = job.check_first;
                new_job.skip_reason = job.skip_reason.clone();
                if let Some(wi) = job.workspace_indicator() {
                    new_job = new_job.with_workspace_indicator(wi.clone());
                }
                batched_jobs.push(new_job);
            }
        }

        batched_jobs
    }
}

impl Step {
    /// Check if this step has any file filters configured.
    ///
    /// Used to determine if an empty file list means "no matching files"
    /// versus "run on all files".
    pub(crate) fn has_filters(&self) -> bool {
        self.glob.is_some()
            || self.dir.is_some()
            || self
                .exclude
                .as_ref()
                .is_some_and(|pattern| !pattern.is_empty())
            || self.types.is_some()
    }
}
