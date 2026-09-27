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
use std::sync::{Arc, OnceLock};

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
            Settings::get().jobs().get(),
        )
    }

    /// Like [`Step::build_step_jobs`], but a batched step splits its files
    /// into at most `batch_jobs` batches instead of `--jobs` (see
    /// [`shared_batch_jobs`]).
    pub(crate) fn build_step_jobs_shared(
        self: &Arc<Self>,
        files: &[PathBuf],
        run_type: RunType,
        files_in_contention: &HashSet<PathBuf>,
        skip_steps: &IndexMap<String, SkipReason>,
        batch_jobs: Option<usize>,
    ) -> Result<Vec<StepJob>> {
        self.build_step_jobs_with_shared(
            self.clone(),
            files,
            run_type,
            files_in_contention,
            skip_steps,
            batch_jobs.unwrap_or_else(|| Settings::get().jobs().get()),
        )
    }

    fn build_step_jobs_with_shared(
        &self,
        shared_step: Arc<Self>,
        files: &[PathBuf],
        run_type: RunType,
        files_in_contention: &HashSet<PathBuf>,
        skip_steps: &IndexMap<String, SkipReason>,
        batch_jobs: usize,
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
        let missing = self.missing_required_env();
        if !missing.is_empty() {
            let mut j = StepJob::new(shared_step, vec![], run_type);
            j.skip_reason = Some(SkipReason::MissingRequiredEnv(missing));
            return Ok(vec![j]);
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
                let counts = batch_counts(&sizes, batch_jobs, self.batch_min_files());
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
            let count = batch_counts(&[files.len()], batch_jobs, self.batch_min_files())[0];
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
            } else if job.check_first && !self.check_is_fix() {
                // Only adjust check_first for jobs where it was already enabled from config
                // Default behavior: only set check_first if there are any files in contention
                // (a step whose check and fix are the same command needs it regardless)
                job.check_first = job.files.iter().any(|f| files_in_contention.contains(f));
            }
        }
        Ok(jobs)
    }
}

/// Fewest files a `batch` step hands to one process, unless the step sets
/// `batch_min_files`.
///
/// Each process pays the tool's startup cost (hundreds of milliseconds for a
/// Node or Python tool), so splitting a handful of files one per process costs
/// more than it saves. pre-commit and prek use the same floor.
const MIN_BATCH_FILES: usize = 4;

impl Step {
    fn batch_min_files(&self) -> usize {
        self.batch_min_files.unwrap_or(MIN_BATCH_FILES).max(1)
    }

    /// Whether this step's batches run alongside the other batched steps of
    /// its group, so that they compete for the same `--jobs`.
    fn shares_batch_jobs(
        &self,
        run_type: RunType,
        skip_steps: &IndexMap<String, SkipReason>,
    ) -> bool {
        self.batch
            && !skip_steps.contains_key(&self.name)
            && self.has_command_for(run_type)
            // A step that waits for others runs when they're done, and one
            // with a condition may not run at all, so they keep every job.
            && self.depends.is_empty()
            && self.step_condition.is_none()
            && self.job_condition.is_none()
            && self.profile_skip_reason().is_none()
            && self.missing_required_env().is_empty()
    }

    /// The variables in `required` that are set neither in the environment
    /// nor in the step's `env`, which make the step skip.
    fn missing_required_env(&self) -> Vec<String> {
        self.required
            .iter()
            .filter(|e| std::env::var(e).is_err() && !self.env.contains_key(*e))
            .cloned()
            .collect()
    }
}

/// The batch counts that the batched steps of one step group share, worked out
/// when the first of them builds its jobs so that the other steps don't wait.
/// Groups run one after another, so each counts the files present when it
/// starts, including any a fixer in an earlier group created.
pub(crate) struct SharedBatchJobs {
    steps: Vec<Arc<Step>>,
    shares: OnceLock<IndexMap<String, usize>>,
}

impl SharedBatchJobs {
    pub(crate) fn new(steps: Vec<Arc<Step>>) -> Self {
        Self {
            steps,
            shares: OnceLock::new(),
        }
    }

    /// How many batches `step` may split its files into, or `None` if it
    /// gets every job.
    pub(crate) fn for_step(
        &self,
        step: &str,
        files: &[PathBuf],
        run_type: RunType,
        skip_steps: &IndexMap<String, SkipReason>,
    ) -> Option<usize> {
        self.shares
            .get_or_init(|| {
                shared_batch_jobs(self.steps.iter().map(|s| &**s), files, run_type, skip_steps)
                    .unwrap_or_else(|err| {
                        // The step reports the error when it filters its files.
                        debug!("not sharing jobs between batched steps: {err:#}");
                        IndexMap::new()
                    })
            })
            .get(step)
            .copied()
    }
}

/// How many batches each batched step of a group may split its files into,
/// for the steps that don't simply get `--jobs`.
///
/// The steps of a group run at the same time. If each batched step split its
/// files into `--jobs` batches, a hook with several of them would start several
/// times as many processes as there are jobs, each paying its tool's startup
/// (hundreds of milliseconds for a Node tool) while the CPUs are already busy.
/// Instead the batched steps share `--jobs` in proportion to their number of
/// files, as the workspaces of one step do, and jobs one step can't use
/// (because of `batch_min_files`) go to the others. A step that is the only
/// batched one with files keeps every job.
///
/// Files are only a rough measure of work: jq formats a JSON file far faster
/// than eslint lints a TypeScript one. So every step first gets an equal share
/// of the jobs, and a step with a few slow files isn't left in one process
/// while quick steps with many files take the rest and finish early. The
/// shares add up to `--jobs` unless there are more such steps than jobs.
fn shared_batch_jobs<'a>(
    steps: impl IntoIterator<Item = &'a Step>,
    files: &[PathBuf],
    run_type: RunType,
    skip_steps: &IndexMap<String, SkipReason>,
) -> Result<IndexMap<String, usize>> {
    let mut names = vec![];
    let mut sizes = vec![];
    let mut caps = vec![];
    for step in steps {
        if !step.shares_batch_jobs(run_type, skip_steps) {
            continue;
        }
        // The same files the step will batch, so a step left with none (all
        // binary, say) takes no share. The binary and symlink checks are
        // cached, so the step doesn't repeat them when it builds its jobs.
        let size = step.filter_files(files)?.len();
        if size == 0 {
            continue;
        }
        names.push(step.name.clone());
        caps.push(batch_cap(size, step.batch_min_files()));
        sizes.push(size);
    }
    if names.len() < 2 {
        return Ok(IndexMap::new());
    }
    let counts = share_jobs(&sizes, &caps, Settings::get().jobs().get(), true);
    Ok(names.into_iter().zip(counts).collect())
}

/// How many batches to split each group of files into (one group per
/// workspace, or a single group), sharing `jobs` between them in proportion to
/// their size. A group never gets so many batches that one would have fewer
/// than `min_files` files, and a non-empty group gets at least one. Jobs a
/// group can't use go to the groups with the largest remaining share.
fn batch_counts(sizes: &[usize], jobs: usize, min_files: usize) -> Vec<usize> {
    let caps: Vec<usize> = sizes
        .iter()
        .map(|&size| batch_cap(size, min_files))
        .collect();
    share_jobs(sizes, &caps, jobs, false)
}

/// The most batches `size` files can make with at least `min_files` in each,
/// and one for a non-empty group with fewer.
fn batch_cap(size: usize, min_files: usize) -> usize {
    (size / min_files.max(1)).max(usize::from(size > 0))
}

/// Shares `jobs` between groups in proportion to `sizes`, giving group `i` no
/// more than `caps[i]` and a non-empty group at least one. With `equal_share`,
/// each non-empty group first gets an equal share of the jobs (as far as its
/// cap allows), and only the rest is shared in proportion to `sizes`.
///
/// The counts add up to at most `jobs`, with one exception: when there are
/// more non-empty groups than jobs, each still gets one.
fn share_jobs(sizes: &[usize], caps: &[usize], jobs: usize, equal_share: bool) -> Vec<usize> {
    let jobs = jobs.max(1);
    let groups = sizes.iter().filter(|&&size| size > 0).count();
    let floor = if equal_share {
        (jobs / groups.max(1)).max(1)
    } else {
        1
    };
    let mut counts: Vec<usize> = sizes
        .iter()
        .zip(caps)
        .map(
            |(&size, &cap)| {
                if size == 0 { 0 } else { floor.min(cap).max(1) }
            },
        )
        .collect();
    // Hand out the remaining jobs one at a time to the group furthest below
    // its proportional share `size * jobs / total` (compared as
    // `size * jobs - count * total` to stay in integers). A group already
    // given more than its share gets more only once the others are capped.
    let total = sizes.iter().sum::<usize>().max(1);
    let mut left = jobs.saturating_sub(counts.iter().sum());
    while left > 0 {
        let next = (0..sizes.len())
            .filter(|&i| counts[i] < caps[i])
            .max_by_key(|&i| (sizes[i] * jobs) as i128 - (counts[i] * total) as i128);
        let Some(i) = next else { break };
        counts[i] += 1;
        left -= 1;
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
        let count = batch_counts(&[files], jobs, MIN_BATCH_FILES)[0];
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
        assert_eq!(batch_counts(&[300, 100], 8, MIN_BATCH_FILES), vec![6, 2]);
        // A small workspace still gets one batch.
        assert_eq!(batch_counts(&[300, 3], 8, MIN_BATCH_FILES), vec![7, 1]);
        // Rounding each share down would leave a job idle here.
        assert_eq!(
            batch_counts(&[50, 50], 3, MIN_BATCH_FILES)
                .iter()
                .sum::<usize>(),
            3
        );
        // An empty workspace gets no batches.
        assert_eq!(batch_counts(&[20, 0], 4, MIN_BATCH_FILES), vec![4, 0]);
        // A small workspace already given a batch above its share (0.84 of 6
        // jobs) doesn't win the leftover job; the 30-file one, furthest below
        // its 1.8, does.
        assert_eq!(
            batch_counts(&[14, 30, 56], 6, MIN_BATCH_FILES),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn workspace_batches_respect_the_minimum_and_the_job_count() {
        for a in 0..60 {
            for b in 0..60 {
                for jobs in 1..=12 {
                    let counts = batch_counts(&[a, b], jobs, MIN_BATCH_FILES);
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
        assert!(
            split_evenly(Vec::<u8>::new(), batch_counts(&[0], 8, MIN_BATCH_FILES)[0]).is_empty()
        );
    }

    #[test]
    fn a_step_can_raise_the_minimum() {
        // prettier's 901 files at 200 per batch: 4 processes on 8 jobs.
        assert_eq!(batch_counts(&[901], 8, 200), vec![4]);
        // Fewer files than the minimum stay in one process.
        assert_eq!(batch_counts(&[62], 8, 200), vec![1]);
        // A minimum of 0 behaves as 1.
        assert_eq!(batch_counts(&[3], 8, 0), vec![3]);
    }

    fn caps(sizes: &[usize], min_files: usize) -> Vec<usize> {
        sizes
            .iter()
            .map(|&size| batch_cap(size, min_files))
            .collect()
    }

    /// Batches per step for steps with `sizes` files sharing 8 jobs.
    fn shared(sizes: &[usize]) -> Vec<usize> {
        share_jobs(sizes, &caps(sizes, MIN_BATCH_FILES), 8, true)
    }

    #[test]
    fn batched_steps_share_the_jobs() {
        // `hk check --all` on the benchmark project: prettier, eslint, jq, yq
        // and shfmt used to start 8 processes each on 8 jobs.
        assert_eq!(shared(&[901, 501, 500, 250, 500]), vec![3, 2, 1, 1, 1]);
        // A 62-file commit: every step already had as few batches as its
        // files allow, so nothing changes.
        assert_eq!(shared(&[9, 5, 5, 3, 5]), vec![2, 1, 1, 1, 1]);
    }

    #[test]
    fn every_step_gets_an_equal_share() {
        // 200 files' share of 8 jobs is 1.3, but an equal share is 2.
        assert_eq!(shared(&[200, 500, 500]), vec![2, 3, 3]);
        // The equal shares already use every job; 901 files' proportional
        // share of 5.1 would take the total to 9.
        assert_eq!(shared(&[901, 501]), vec![4, 4]);
        // A step with more files than its equal share gets the jobs left.
        assert_eq!(shared(&[901, 100, 100]), vec![4, 2, 2]);
        // Steps with few files still get their equal share, as far as
        // batch_min_files allows.
        assert_eq!(shared(&[4000, 8, 8]), vec![4, 2, 2]);
    }

    #[test]
    fn jobs_a_step_cant_use_go_to_the_others() {
        // The first step's batch_min_files keeps it in one process.
        assert_eq!(share_jobs(&[400, 400], &[1, 100], 8, true), vec![1, 7]);
    }

    #[test]
    fn shares_stay_within_the_job_count() {
        let mixes: &[&[usize]] = &[
            &[901, 501],
            &[901, 501, 500, 250, 500],
            &[200, 500, 500],
            &[1, 1000],
            &[3, 5, 7, 11, 13, 17, 19, 23, 29],
            &[4000, 4, 4, 4],
            &[10, 10, 1, 1],
            &[30, 1, 1],
        ];
        for &sizes in mixes {
            for jobs in 1..=40 {
                for min_files in [1, MIN_BATCH_FILES, 50, 200] {
                    let caps = caps(sizes, min_files);
                    for equal_share in [false, true] {
                        let counts = share_jobs(sizes, &caps, jobs, equal_share);
                        let at = format!("{sizes:?} on {jobs} jobs (min {min_files}): {counts:?}");
                        let steps = sizes.iter().filter(|&&s| s > 0).count();
                        // The one exception: more steps than jobs, one each.
                        assert!(counts.iter().sum::<usize>() <= jobs.max(steps), "{at}");
                        // Every job is used unless the caps don't allow it.
                        assert_eq!(
                            counts.iter().sum::<usize>(),
                            jobs.max(steps).min(caps.iter().sum()),
                            "{at}"
                        );
                        for (&cap, &count) in caps.iter().zip(&counts) {
                            assert!(count >= 1 && count <= cap, "{at}");
                            if equal_share {
                                assert!(count >= (jobs / steps).min(cap), "{at}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn zero_jobs_is_treated_as_one() {
        assert_eq!(sizes(10, 0), vec![10]);
    }
}
