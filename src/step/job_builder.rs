//! Step job creation and configuration.
//!
//! This module is responsible for creating [`StepJob`]s from a step configuration
//! and a list of files. It handles:
//!
//! - File filtering
//! - Workspace splitting (for monorepos)
//! - Batch mode job creation
//! - Skip reason determination
//! - Check-first mode configuration

use crate::Result;
use crate::hook::SkipReason;
use crate::settings::Settings;
use crate::step_job::StepJob;
use indexmap::IndexMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use super::types::{RunType, Step};

impl Step {
    /// Create step jobs from a list of files.
    ///
    /// This is the main entry point for creating executable jobs from a step.
    /// It applies all filtering, batching, and skip logic to produce jobs
    /// ready for execution.
    ///
    /// # Job Creation Process
    ///
    /// 1. Check for explicit skip (via skip_steps)
    /// 2. Check if step has a command for the run type
    /// 3. Filter files based on step configuration
    /// 4. Split into workspace jobs (if workspace_indicator set) or batch jobs
    /// 5. Apply auto-batching for ARG_MAX safety
    /// 6. Configure check_first based on file contention
    ///
    /// # Arguments
    ///
    /// * `files` - All files to consider
    /// * `run_type` - Whether running check or fix
    /// * `files_in_contention` - Files being modified by other steps (for check_first)
    /// * `skip_steps` - Steps explicitly marked to skip
    ///
    /// # Returns
    ///
    /// A list of jobs ready for execution
    pub(crate) fn build_step_jobs(
        &self,
        files: &[PathBuf],
        run_type: RunType,
        files_in_contention: &HashSet<PathBuf>,
        skip_steps: &IndexMap<String, SkipReason>,
    ) -> Result<Vec<StepJob>> {
        // Pre-calculate skip reason at the job creation level to simplify run_all_jobs
        if skip_steps.contains_key(&self.name) {
            let reason = skip_steps.get(&self.name).unwrap().clone();
            let mut j = StepJob::new(Arc::new(self.clone()), vec![], run_type);
            j.skip_reason = Some(reason);
            return Ok(vec![j]);
        }
        if !self.has_command_for(run_type) {
            let mut j = StepJob::new(Arc::new(self.clone()), vec![], run_type);
            j.skip_reason = Some(SkipReason::NoCommandForRunType(run_type));
            return Ok(vec![j]);
        }
        let files = self.filter_files(files)?;
        // Skip if no files and step has file filters
        // This means the step was explicitly looking for specific files and found none
        if files.is_empty() && self.has_filters() {
            debug!("{self}: no file matches for step");
            let mut j = StepJob::new(Arc::new(self.clone()), vec![], run_type);
            j.skip_reason = Some(SkipReason::NoFilesToProcess);
            return Ok(vec![j]);
        }
        let mut jobs = if let Some(workspace_indicators) = self.workspaces_for_files(&files)? {
            let mut files = files.clone();

            workspace_indicators
                // Sort the files in reverse so the longest directory can take files in their directories
                // and then the shortest path will take the rest of them.
                .sorted_by(|a, b| b.as_os_str().len().cmp(&a.as_os_str().len()))
                .map(|workspace_indicator| {
                    let workspace_dir = workspace_indicator.parent();
                    let mut workspace_files = Vec::new();
                    let mut i = 0;

                    while i < files.len() {
                        if workspace_dir
                            .map(|dir| files[i].starts_with(dir))
                            .unwrap_or(true)
                        {
                            let val = files.remove(i);
                            workspace_files.push(val);
                        } else {
                            i += 1;
                        }
                    }

                    StepJob::new(Arc::new((*self).clone()), workspace_files, run_type)
                        .with_workspace_indicator(workspace_indicator)
                })
                .collect()
        } else if self.batch {
            files
                .chunks((files.len() / Settings::get().jobs().get()).max(1))
                .map(|chunk| StepJob::new(Arc::new((*self).clone()), chunk.to_vec(), run_type))
                .collect()
        } else {
            vec![StepJob::new(
                Arc::new((*self).clone()),
                files.clone(),
                run_type,
            )]
        };

        if self.stdin.is_none() {
            // Auto-batch any jobs where the file list would exceed safe limits
            jobs = self.auto_batch_jobs_if_needed(jobs);
        }

        // Apply profile skip only after determining files/no-files, so NoFilesToProcess wins
        // Also, if a condition is present, defer profile checks to run() so ConditionFalse wins
        if self.job_condition.is_none()
            && let Some(reason) = self.profile_skip_reason()
        {
            for job in jobs.iter_mut() {
                job.skip_reason = Some(reason.clone());
            }
        }
        // If stage=<JOB_FILES> and check_list_files or check_diff is defined, always run check_first
        // to ensure files are filtered correctly, even when there's no contention
        let needs_filtering_for_stage = self
            .stage
            .as_ref()
            .map(|v| v.len() == 1 && v[0] == "<JOB_FILES>")
            .unwrap_or(false)
            && (self.check_list_files.is_some() || self.check_diff.is_some());

        // Always run check_first when check_diff is defined so we can apply the diff directly
        let can_apply_diff = self.check_diff.is_some();

        for job in jobs.iter_mut() {
            if needs_filtering_for_stage || can_apply_diff {
                // Always run check_first when we need to filter files for stage=<JOB_FILES>
                // or when we can apply the diff directly
                job.check_first = true;
            } else if job.check_first {
                // Only adjust check_first for jobs where it was already enabled from config
                // Default behavior: only set check_first if there are any files in contention
                job.check_first = job.files.iter().any(|f| files_in_contention.contains(f));
            }
        }
        Ok(jobs)
    }
}
