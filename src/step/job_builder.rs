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

use super::types::{CheckFirstCmd, RunType, Step};

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
    /// 5. Configure check_first based on file contention
    ///
    /// Auto-batching for ARG_MAX safety is applied later by [`Step::auto_batch_jobs`]
    /// (called from execution time) so the full tera context is available to render
    /// the actual command.
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
        self.build_step_jobs_with_shared(
            Arc::new(self.clone()),
            files,
            run_type,
            files_in_contention,
            skip_steps,
        )
    }

    pub(crate) fn build_step_jobs_shared(
        self: &Arc<Self>,
        files: &[PathBuf],
        run_type: RunType,
        files_in_contention: &HashSet<PathBuf>,
        skip_steps: &IndexMap<String, SkipReason>,
    ) -> Result<Vec<StepJob>> {
        self.build_step_jobs_with_shared(
            self.clone(),
            files,
            run_type,
            files_in_contention,
            skip_steps,
        )
    }

    fn build_step_jobs_with_shared(
        &self,
        shared_step: Arc<Self>,
        files: &[PathBuf],
        run_type: RunType,
        files_in_contention: &HashSet<PathBuf>,
        skip_steps: &IndexMap<String, SkipReason>,
    ) -> Result<Vec<StepJob>> {
        // Pre-calculate skip reason at the job creation level to simplify run_all_jobs
        if skip_steps.contains_key(&self.name) {
            let reason = skip_steps.get(&self.name).unwrap().clone();
            let mut j = StepJob::new(shared_step, vec![], run_type);
            j.skip_reason = Some(reason);
            return Ok(vec![j]);
        }
        if !self.has_command_for(run_type) {
            let mut j = StepJob::new(shared_step, vec![], run_type);
            j.skip_reason = Some(SkipReason::NoCommandForRunType(run_type));
            return Ok(vec![j]);
        }
        if !self.required.is_empty() {
            let missing: Vec<String> = self
                .required
                .iter()
                .filter(|e| std::env::var(e).is_err() && !self.env.contains_key(*e))
                .cloned()
                .collect();
            if !missing.is_empty() {
                let mut j = StepJob::new(shared_step, vec![], run_type);
                j.skip_reason = Some(SkipReason::MissingRequiredEnv(missing));
                return Ok(vec![j]);
            }
        }
        let files = self.filter_files(files)?;
        // Skip if no files and step has file filters
        // This means the step was explicitly looking for specific files and found none
        if files.is_empty() && self.has_filters() {
            debug!("{self}: no file matches for step");
            let mut j = StepJob::new(shared_step, vec![], run_type);
            j.skip_reason = Some(SkipReason::NoFilesToProcess);
            return Ok(vec![j]);
        }
        let mut jobs = if let Some(workspace_indicators) = self.workspaces_for_files(&files)? {
            let mut files = files.clone();
            let groups: Vec<_> = workspace_indicators
                // Sort the files in reverse so the longest directory can take files in their directories
                // and then the shortest path will take the rest of them.
                .sorted_by(|a, b| b.as_os_str().len().cmp(&a.as_os_str().len()))
                .map(|workspace_indicator| {
                    let workspace_dir = workspace_indicator.parent();
                    let remaining = std::mem::take(&mut files);
                    let (workspace_files, other_files): (Vec<_>, Vec<_>) =
                        remaining.into_iter().partition(|file| {
                            workspace_dir
                                .map(|dir| file.starts_with(dir))
                                .unwrap_or(true)
                        });
                    files = other_files;
                    (workspace_indicator, workspace_files)
                })
                .collect();

            if self.batch {
                // Share the job count across workspaces, so the total number of
                // jobs stays ~jobs, not jobs per workspace.
                let sizes: Vec<usize> = groups.iter().map(|(_, f)| f.len()).collect();
                let counts = batch_counts(&sizes, Settings::get().jobs().get());
                groups
                    .into_iter()
                    .zip(counts)
                    .flat_map(|((workspace_indicator, workspace_files), count)| {
                        let shared_step = shared_step.clone();
                        split_evenly(workspace_files, count)
                            .into_iter()
                            .map(move |chunk| {
                                StepJob::new(shared_step.clone(), chunk, run_type)
                                    .with_workspace_indicator(workspace_indicator.clone())
                            })
                    })
                    .collect()
            } else {
                groups
                    .into_iter()
                    .map(|(workspace_indicator, workspace_files)| {
                        StepJob::new(shared_step.clone(), workspace_files, run_type)
                            .with_workspace_indicator(workspace_indicator)
                    })
                    .collect()
            }
        } else if self.batch {
            let count = batch_counts(&[files.len()], Settings::get().jobs().get())[0];
            split_evenly(files.clone(), count)
                .into_iter()
                .map(|chunk| StepJob::new(shared_step.clone(), chunk, run_type))
                .collect()
        } else {
            vec![StepJob::new(shared_step, files.clone(), run_type)]
        };

        // Note: auto-batching for ARG_MAX safety happens at execution time
        // (see `Step::auto_batch_jobs`) where the full tera context is available
        // to render the actual run command — so steps whose commands don't
        // reference `{{files}}` are not split into many jobs unnecessarily.

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

        // In Fix mode, run check_first when check_diff is defined so we can apply the diff directly.
        // In Check mode, this is avoided as check_diff may hide non-auto-fixable errors.
        let can_apply_diff = self.check_diff.is_some() && matches!(run_type, RunType::Fix);

        // Optionally use the list/diff command to focus the regular check on
        // only the files that failed. This is opt-in because it adds a second
        // tool invocation and not every check command accepts file arguments.
        let needs_focused_check = self.check_failed_files
            && matches!(run_type, RunType::Check)
            && matches!(
                self.check_first_cmd(),
                Some(CheckFirstCmd::Diff(_) | CheckFirstCmd::ListFiles(_))
            );

        for job in jobs.iter_mut() {
            if needs_filtering_for_stage || can_apply_diff || needs_focused_check {
                // Always run check_first when we need to filter files for stage=<JOB_FILES>
                // or when we can apply the diff directly or focus a check
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

/// Fewest files a `batch` step hands to one process.
///
/// Each process pays the tool's startup cost (hundreds of milliseconds for a
/// Node or Python tool), so splitting a handful of files one per process costs
/// more than it saves. pre-commit and prek use the same floor.
const MIN_BATCH_FILES: usize = 4;

/// How many batches to split each group of files into (one group per
/// workspace, or a single group), sharing `jobs` between them in proportion to
/// their size. A group never gets so many batches that one would have fewer
/// than [`MIN_BATCH_FILES`] files, and a non-empty group gets at least one.
/// Jobs a group can't use go to the groups with the largest remaining share.
fn batch_counts(sizes: &[usize], jobs: usize) -> Vec<usize> {
    let jobs = jobs.max(1);
    let total: usize = sizes.iter().sum();
    let cap = |size: usize| (size / MIN_BATCH_FILES).max(usize::from(size > 0));
    let mut counts: Vec<usize> = sizes
        .iter()
        .map(|&size| {
            (size * jobs / total.max(1))
                .min(cap(size))
                .max(usize::from(size > 0))
        })
        .collect();
    // Hand out the jobs rounding down left over, largest remainder first.
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse((sizes[i] * jobs) % total.max(1)));
    let mut left = jobs.saturating_sub(counts.iter().sum());
    while left > 0 {
        let before = left;
        for &i in &order {
            if left > 0 && counts[i] < cap(sizes[i]) {
                counts[i] += 1;
                left -= 1;
            }
        }
        if left == before {
            break;
        }
    }
    counts
}

/// Split `files` into `count` batches whose sizes differ by at most one, so no
/// batch is left with a short remainder. No files make no batches, so the step
/// is reported as having nothing to process.
fn split_evenly<T>(files: Vec<T>, count: usize) -> Vec<Vec<T>> {
    if files.is_empty() {
        return vec![];
    }
    let count = count.clamp(1, files.len());
    let (base, extra) = (files.len() / count, files.len() % count);
    let mut files = files.into_iter();
    (0..count)
        .map(|i| files.by_ref().take(base + usize::from(i < extra)).collect())
        .collect()
}

#[cfg(test)]
mod batch_tests {
    use super::*;

    fn sizes(files: usize, jobs: usize) -> Vec<usize> {
        let count = batch_counts(&[files], jobs)[0];
        split_evenly((0..files).collect::<Vec<_>>(), count)
            .iter()
            .map(Vec::len)
            .collect()
    }

    #[test]
    fn a_few_files_share_one_process() {
        // 5 files on 8 jobs used to start 5 processes, each paying the tool's
        // startup cost; a floor-sized chunk would still leave a 1-file batch.
        assert_eq!(sizes(5, 8), vec![5]);
        assert_eq!(sizes(1, 8), vec![1]);
        assert_eq!(sizes(9, 8), vec![5, 4]);
    }

    #[test]
    fn every_batch_gets_at_least_the_minimum() {
        for files in MIN_BATCH_FILES..200 {
            for jobs in 1..=16 {
                let sizes = sizes(files, jobs);
                assert!(sizes.len() <= jobs, "{files} files, {jobs} jobs: {sizes:?}");
                assert!(
                    sizes.iter().all(|&n| n >= MIN_BATCH_FILES),
                    "{files} files, {jobs} jobs: {sizes:?}"
                );
                assert_eq!(sizes.iter().sum::<usize>(), files);
            }
        }
    }

    #[test]
    fn many_files_spread_over_every_job() {
        assert_eq!(sizes(4000, 8), vec![500; 8]);
        assert_eq!(sizes(4001, 8).len(), 8);
        assert_eq!(sizes(8, 2), vec![4, 4]);
    }

    #[test]
    fn workspaces_share_the_job_count() {
        // 8 jobs over 400 files: a 300-file workspace gets 6, a 100-file one 2.
        assert_eq!(batch_counts(&[300, 100], 8), vec![6, 2]);
        // A small workspace still gets one batch.
        assert_eq!(batch_counts(&[300, 3], 8), vec![7, 1]);
        // Rounding each share down would leave a job idle here.
        assert_eq!(batch_counts(&[50, 50], 3).iter().sum::<usize>(), 3);
        // An empty workspace gets no batches.
        assert_eq!(batch_counts(&[20, 0], 4), vec![4, 0]);
    }

    #[test]
    fn workspace_batches_respect_the_minimum_and_the_job_count() {
        for a in 0..60 {
            for b in 0..60 {
                for jobs in 1..=12 {
                    let counts = batch_counts(&[a, b], jobs);
                    for (size, count) in [a, b].into_iter().zip(&counts) {
                        assert!(size == 0 || *count >= 1, "{a},{b} on {jobs}: {counts:?}");
                        assert!(*count <= (size / MIN_BATCH_FILES).max(usize::from(size > 0)));
                    }
                    let groups = counts.iter().filter(|&&c| c > 0).count();
                    assert!(counts.iter().sum::<usize>() <= jobs.max(groups));
                }
            }
        }
    }

    #[test]
    fn no_files_make_no_batches() {
        assert!(split_evenly(Vec::<u8>::new(), batch_counts(&[0], 8)[0]).is_empty());
    }

    #[test]
    fn zero_jobs_is_treated_as_one() {
        assert_eq!(sizes(10, 0), vec![10]);
    }
}
