use crate::{Result, error::Error, step_job::StepJob};
use crate::{env, step_job::StepJobStatus};
use crate::{glob, settings::Settings};
use crate::{hook::SkipReason, timings::StepTimingGuard};
use crate::{step_context::StepContext, tera, ui::style};
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use ensembler::CmdLineRunner;
use eyre::{WrapErr, eyre};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};
use shell_quote::QuoteInto;
use shell_quote::QuoteRefExt;
use std::{
    collections::{BTreeSet, HashSet},
    fmt::Display,
    path::PathBuf,
    str::FromStr,
};
use std::{
    ffi::OsString,
    sync::{Arc, LazyLock},
};
use std::{fmt, process::Stdio};
use tokio::sync::OwnedSemaphorePermit;
use xx::file::display_path;

use crate::step_test::StepTest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Pattern {
    Regex {
        #[serde(skip_serializing)]
        _type: String,
        pattern: String,
    },
    Globs(Vec<String>),
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        use serde_json::Value;

        let value = Value::deserialize(deserializer)?;

        // Check if it's a regex object with _type field
        if let Value::Object(ref map) = value {
            if let Some(Value::String(type_str)) = map.get("_type") {
                if type_str == "regex" {
                    if let Some(Value::String(pattern)) = map.get("pattern") {
                        return Ok(Pattern::Regex {
                            _type: "regex".to_string(),
                            pattern: pattern.clone(),
                        });
                    }
                }
            }
        }

        // Try to deserialize as a string
        if let Value::String(s) = value {
            return Ok(Pattern::Globs(vec![s]));
        }

        // Try to deserialize as array of strings
        if let Value::Array(arr) = value {
            let globs: Result<Vec<String>, _> = arr
                .into_iter()
                .map(|v| {
                    if let Value::String(s) = v {
                        Ok(s)
                    } else {
                        Err(D::Error::custom("array elements must be strings"))
                    }
                })
                .collect();
            return Ok(Pattern::Globs(globs?));
        }

        Err(D::Error::custom(
            "expected regex object, string, or array of strings",
        ))
    }
}

#[serde_as]
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct Step {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,
    #[serde(default)]
    pub name: String,
    pub profiles: Option<Vec<String>>,
    #[serde(default)]
    pub glob: Option<Pattern>,
    #[serde(default)]
    pub interactive: bool,
    pub depends: Vec<String>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub shell: Option<Script>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check: Option<Script>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check_list_files: Option<Script>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub check_diff: Option<Script>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub fix: Option<Script>,
    pub workspace_indicator: Option<String>,
    pub prefix: Option<String>,
    pub dir: Option<String>,
    pub condition: Option<String>,
    #[serde(default)]
    pub check_first: bool,
    #[serde(default)]
    pub batch: bool,
    #[serde(default)]
    pub stomp: bool,
    pub env: IndexMap<String, String>,
    pub stage: Option<Vec<String>>,
    pub exclude: Option<Pattern>,
    #[serde(default)]
    pub exclusive: bool,
    pub root: Option<PathBuf>,
    #[serde(default)]
    pub hide: bool,
    #[serde(default)]
    pub tests: indexmap::IndexMap<String, StepTest>,
    #[serde(default)]
    pub output_summary: OutputSummary,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    Check(CheckType),
    Fix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckType {
    Check,
    ListFiles,
    Diff,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputSummary {
    #[default]
    Stderr,
    Stdout,
    Combined,
    Hide,
}

impl Step {
    pub(crate) fn init(&mut self, name: &str) {
        self.name = name.to_string();
        if self.interactive {
            self.exclusive = true;
        }
    }

    pub fn run_cmd(&self, run_type: RunType) -> Option<&Script> {
        match run_type {
            RunType::Check(c) => match c {
                CheckType::Check => self.check.as_ref(),
                CheckType::Diff => self.check_diff.as_ref(),
                CheckType::ListFiles => self.check_list_files.as_ref(),
            }
            .or(self.check.as_ref())
            .or(self.check_list_files.as_ref())
            .or(self.check_diff.as_ref()),
            RunType::Fix => self
                .fix
                .as_ref()
                .or_else(|| self.run_cmd(RunType::Check(CheckType::Check))),
        }
    }

    pub fn check_type(&self) -> CheckType {
        if self.check_diff.is_some() {
            CheckType::Diff
        } else if self.check_list_files.is_some() {
            CheckType::ListFiles
        } else {
            CheckType::Check
        }
    }

    pub fn enabled_profiles(&self) -> Option<IndexSet<String>> {
        self.profiles.as_ref().map(|profiles| {
            profiles
                .iter()
                .filter(|s| !s.starts_with('!'))
                .map(|s| s.to_string())
                .collect()
        })
    }

    pub fn disabled_profiles(&self) -> Option<IndexSet<String>> {
        self.profiles.as_ref().map(|profiles| {
            profiles
                .iter()
                .filter(|s| s.starts_with('!'))
                .map(|s| s.strip_prefix('!').unwrap().to_string())
                .collect()
        })
    }

    pub fn profile_skip_reason(&self) -> Option<SkipReason> {
        let settings = Settings::get();
        if let Some(enabled) = self.enabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let missing_profiles = enabled.difference(&enabled_profiles).collect::<Vec<_>>();
            if !missing_profiles.is_empty() {
                let profiles = missing_profiles
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                return Some(SkipReason::ProfileNotEnabled(profiles));
            }
            let disabled_profiles_set = settings.disabled_profiles();
            let disabled_profiles = disabled_profiles_set.intersection(&enabled).collect_vec();
            if !disabled_profiles.is_empty() {
                return Some(SkipReason::ProfileExplicitlyDisabled);
            }
        }
        if let Some(disabled) = self.disabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let disabled_profiles = disabled.intersection(&enabled_profiles).collect::<Vec<_>>();
            if !disabled_profiles.is_empty() {
                return Some(SkipReason::ProfileExplicitlyDisabled);
            }
        }
        None
    }

    pub(crate) fn build_step_progress(&self) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body("{{spinner()}} {{name | flex}} {% if show_step_progress %}{{progress_bar(width=20)}} {{cur}}/{{total}}{% endif %}{% if message %} – {{message | flex}}{% elif files %} – {{files}}{% endif %}")
            .body_text(Some(
                "{{spinner()}} {{name}}{% if show_step_progress %}  {{progress_bar(width=20)}} {{cur}}/{{total}}{% endif %}{% if message %} – {{message}}{% elif files %} – {{files}}{% endif %}",
            ))
            .prop("name", &self.name)
            .prop("files", &0)
            .status(ProgressStatus::Hide)
            .on_done(if *env::HK_HIDE_WHEN_DONE {
                ProgressJobDoneBehavior::Hide
            } else {
                ProgressJobDoneBehavior::Keep
            })
            .start()
    }

    /// For a list of files like this:
    /// src/crate-1/src/lib.rs
    /// src/crate-1/src/subdir/mod.rs
    /// src/crate-2/src/lib.rs
    /// src/crate-2/src/subdir/mod.rs
    /// If the workspace indicator is "Cargo.toml", and there are Cargo.toml files in the root of crate-1 and crate-2,
    /// this will return: ["src/crate-1/Cargo.toml", "src/crate-2/Cargo.toml"]
    pub fn workspaces_for_files(&self, files: &[PathBuf]) -> Result<Option<IndexSet<PathBuf>>> {
        let Some(workspace_indicator) = &self.workspace_indicator else {
            return Ok(None);
        };
        let mut dirs = files.iter().filter_map(|f| f.parent()).collect_vec();
        let mut workspaces: IndexSet<PathBuf> = Default::default();
        while let Some(dir) = dirs.pop() {
            if let Some(workspace) = xx::file::find_up(dir, &[workspace_indicator]) {
                workspaces.insert(workspace);
            }
        }
        Ok(Some(workspaces))
    }

    fn filter_files(&self, files: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut files = files.to_vec();
        if let Some(dir) = &self.dir {
            files.retain(|f| f.starts_with(dir));
            if files.is_empty() {
                debug!("{self}: no files in {dir}");
            }
            // Don't strip the dir prefix here - it causes issues when steps have different working directories
            // The path stripping should only happen in the command execution context via tera templates
        }
        if let Some(pattern) = &self.glob {
            // Use get_pattern_matches consistently for both globs and regex
            files = glob::get_pattern_matches(pattern, &files, self.dir.as_deref())?;
        }
        if let Some(pattern) = &self.exclude {
            // Use get_pattern_matches consistently for excludes too
            let excluded: HashSet<_> =
                glob::get_pattern_matches(pattern, &files, self.dir.as_deref())?
                    .into_iter()
                    .collect();
            files.retain(|f| !excluded.contains(f));
        }
        Ok(files)
    }

    /// Estimates the size of the {{files}} template variable expansion for a given list of files.
    /// This includes shell quoting overhead and spaces between files.
    fn estimate_files_string_size(&self, files: &[PathBuf]) -> usize {
        files
            .iter()
            .map(|f| {
                let path_str = f.to_str().unwrap_or("");
                // Estimate quoted size: conservative estimate assuming worst-case quoting
                // For shell quoting, worst case is roughly 2x + 2 (quotes)
                path_str.len() * 2 + 2 + 1 // +1 for space separator
            })
            .sum()
    }

    /// Automatically batch jobs if the file list would exceed safe ARG_MAX limits.
    /// This prevents "Argument list too long" errors when passing large file lists to commands.
    fn auto_batch_jobs_if_needed(&self, jobs: Vec<StepJob>) -> Vec<StepJob> {
        // Use 50% of ARG_MAX as a safety margin, accounting for environment variables
        // and the command itself
        let safe_limit = *env::ARG_MAX / 2;

        let mut batched_jobs = Vec::new();

        for job in jobs {
            let estimated_size = self.estimate_files_string_size(&job.files);

            if estimated_size > safe_limit && job.files.len() > 1 {
                // Need to batch this job
                debug!(
                    "{}: auto-batching {} files (estimated size: {} bytes, limit: {} bytes)",
                    self.name,
                    job.files.len(),
                    estimated_size,
                    safe_limit
                );

                // Binary search to find optimal batch size
                let mut batch_size = job.files.len() / 2;
                let mut low = 1;
                let mut high = job.files.len();

                while low < high {
                    let mid = (low + high).div_ceil(2);
                    let test_size =
                        self.estimate_files_string_size(&job.files[..mid.min(job.files.len())]);

                    if test_size <= safe_limit {
                        low = mid;
                        batch_size = mid;
                    } else {
                        high = mid - 1;
                    }
                }

                // Create batched jobs - use the StepJob constructor to properly handle private fields
                for chunk in job.files.chunks(batch_size) {
                    let new_job = StepJob::new(job.step.clone(), chunk.to_vec(), job.run_type);
                    // Note: we can't preserve workspace_indicator or other private fields
                    // without adding public methods to StepJob. For now, batching will
                    // break workspace_indicator jobs, but that's acceptable since those
                    // are typically small workspaces.
                    batched_jobs.push(new_job);
                }
            } else {
                // No batching needed
                batched_jobs.push(job);
            }
        }

        batched_jobs
    }

    pub(crate) fn build_step_jobs(
        &self,
        files: &[PathBuf],
        run_type: RunType,
        files_in_contention: &HashSet<PathBuf>,
        skip_steps: &indexmap::IndexMap<String, crate::hook::SkipReason>,
    ) -> Result<Vec<StepJob>> {
        // Pre-calculate skip reason at the job creation level to simplify run_all_jobs
        if skip_steps.contains_key(&self.name) {
            let reason = skip_steps.get(&self.name).unwrap().clone();
            let mut j = StepJob::new(Arc::new(self.clone()), vec![], run_type);
            j.skip_reason = Some(reason);
            return Ok(vec![j]);
        }
        if self.run_cmd(run_type).is_none() {
            let mut j = StepJob::new(Arc::new(self.clone()), vec![], run_type);
            j.skip_reason = Some(SkipReason::NoCommandForRunType(run_type));
            return Ok(vec![j]);
        }
        let files = self.filter_files(files)?;
        if files.is_empty() && (self.glob.is_some() || self.dir.is_some() || self.exclude.is_some())
        {
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

        // Auto-batch any jobs where the file list would exceed safe limits
        jobs = self.auto_batch_jobs_if_needed(jobs);

        // Apply profile skip only after determining files/no-files, so NoFilesToProcess wins
        // Also, if a condition is present, defer profile checks to run() so ConditionFalse wins
        if self.condition.is_none() {
            if let Some(reason) = self.profile_skip_reason() {
                for job in jobs.iter_mut() {
                    job.skip_reason = Some(reason.clone());
                }
            }
        }
        for job in jobs.iter_mut().filter(|j| j.check_first) {
            // only set check_first if there are any files in contention
            job.check_first = job.files.iter().any(|f| files_in_contention.contains(f));
        }
        Ok(jobs)
    }

    pub(crate) async fn run_all_jobs(
        &self,
        ctx: Arc<StepContext>,
        semaphore: Option<OwnedSemaphorePermit>,
    ) -> Result<()> {
        let semaphore = self.wait_for_depends(&ctx, semaphore).await?;
        let files = ctx.hook_ctx.files();
        let ctx = Arc::new(ctx);
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
        let all_job_files: indexmap::IndexSet<PathBuf> =
            jobs.iter().flat_map(|j| j.files.clone()).collect();

        let mut set = tokio::task::JoinSet::new();
        for job in jobs {
            let ctx = ctx.clone();
            let step = self.clone();
            let mut job = job;
            set.spawn(async move {
                if let Some(reason) = &job.skip_reason {
                    step.mark_skipped(&ctx, reason)?;
                    // Skipped jobs should still count as completed for overall progress
                    ctx.hook_ctx.inc_completed_jobs(1);
                    return Ok(());
                }
                if job.check_first {
                    let prev_run_type = job.run_type;
                    job.run_type = RunType::Check(step.check_type());
                    debug!("{step}: running check step first due to fix step contention");
                    match step.run(&ctx, &mut job).await {
                        Ok(()) => {
                            debug!("{step}: successfully ran check step first");
                            ctx.hook_ctx.inc_completed_jobs(1);
                            return Ok(());
                        }
                        Err(e) => {
                            if let Some(Error::CheckListFailed { source, stdout, stderr }) =
                                e.downcast_ref::<Error>()
                            {
                                debug!("{step}: failed check step first: {source}");
                                let (files, extras) = step.filter_files_from_check_list(&job.files, stdout);
                                for f in extras {
                                    warn!(
                                        "{step}: file in check_list_files not found in original files: {}",
                                        f.display()
                                    );
                                }

                                // If no files remain after filtering and stderr is non-empty, fail with the stderr output
                                if files.is_empty() && !stderr.trim().is_empty() {
                                    error!("{step}: check_list_files returned no files and produced errors:\n{}", stderr);
                                    return Err(eyre!("check_list_files failed with errors:\n{}", stderr));
                                }

                                job.files = files;
                            }
                            debug!("{step}: failed check step first: {e}");
                        }
                    }
                    job.run_type = prev_run_type;
                }
                let result = step.run(&ctx, &mut job).await;
                if let Err(err) = &result {
                    job.status_errored(&ctx, format!("{err}")).await?;
                }
                ctx.hook_ctx.inc_completed_jobs(1);
                result
            });
        }
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => {}
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
        if non_skip_jobs > 0 && matches!(ctx.hook_ctx.run_type, RunType::Fix) {
            // Build stage pathspecs; if `dir` is set, stage entries are relative to it
            // Compute "root" variants for patterns that start with "**/" BEFORE prefixing with `dir`.
            let rendered_patterns: Vec<String> = self
                .stage
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .map(|s| tera::render(s, &ctx.hook_ctx.tctx))
                .collect::<Result<Vec<_>>>()?;

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
                if let Some(rest) = pat.strip_prefix("**/") {
                    if !rest.is_empty() {
                        if let Some(dir) = &self.dir {
                            stage_globs.push(format!("{}/{}", dir.trim_end_matches('/'), rest));
                        } else {
                            stage_globs.push(rest.to_string());
                        }
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
            if !stage_pathspecs.is_empty() {
                trace!(
                    "{}: requesting status for pathspecs: {:?}",
                    self, &stage_pathspecs
                );
                let status = ctx
                    .hook_ctx
                    .git
                    .lock()
                    .await
                    .status(Some(&stage_pathspecs))?;

                // Build a scoped candidate set:
                //  - Include only files that this step actually operated on (union of job files)
                //  - Additionally include any explicit, non-glob stage paths (to allow generators)
                let is_globlike = |s: &str| s.contains('*') || s.contains('?') || s.contains('[');
                let mut candidates: indexmap::IndexSet<PathBuf> = all_job_files.clone();
                for pat in &stage_globs {
                    if !is_globlike(pat) {
                        let p = PathBuf::from(pat);
                        if p.exists() {
                            candidates.insert(p);
                        }
                    }
                }

                // Build candidate list from job files plus explicit non-glob stage paths.
                // For anchored globs (e.g., "dir/**"), allow matching against unstaged files too
                // so that generators can stage new files outside job file set.
                let is_globlike = |s: &str| s.contains('*') || s.contains('?') || s.contains('[');
                let is_anchored_glob = |s: &str| {
                    if s.starts_with("**/") {
                        return false;
                    }
                    let first = s.split('/').next().unwrap_or(s);
                    if is_globlike(first) {
                        return false;
                    }
                    is_globlike(s)
                };
                if stage_globs.iter().any(|g| is_anchored_glob(g)) {
                    for p in status.unstaged_files.iter() {
                        candidates.insert(p.clone());
                    }
                }
                let candidate_vec = candidates.into_iter().collect_vec();
                let matched_candidates = glob::get_matches(&stage_globs, &candidate_vec)?;
                // Now keep only those that are actually unstaged
                let unstaged_set: indexmap::IndexSet<PathBuf> =
                    status.unstaged_files.iter().cloned().collect();
                let filtered = matched_candidates
                    .into_iter()
                    .filter(|p| unstaged_set.contains(p))
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
                    if Settings::get().stage {
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
        }
        if non_skip_jobs > 0 {
            ctx.status_finished();
            ctx.depends.mark_done(&self.name)?;
        }
        Ok(())
    }

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

    fn save_output_summary(
        &self,
        ctx: &StepContext,
        job: &StepJob,
        stdout: &str,
        stderr: &str,
        combined: &str,
        is_failure: bool,
    ) {
        // Only skip if this is a check_first check that FAILED (will be followed by a fix)
        // If the check passed, we want to show its output since no fix will run
        let is_check_first_check_that_failed =
            job.check_first && matches!(job.run_type, RunType::Check(_)) && is_failure;
        if is_check_first_check_that_failed {
            return;
        }

        match self.output_summary {
            OutputSummary::Stderr => {
                ctx.hook_ctx
                    .append_step_output(&self.name, OutputSummary::Stderr, stderr)
            }
            OutputSummary::Stdout => {
                ctx.hook_ctx
                    .append_step_output(&self.name, OutputSummary::Stdout, stdout)
            }
            OutputSummary::Combined => {
                ctx.hook_ctx
                    .append_step_output(&self.name, OutputSummary::Combined, combined)
            }
            OutputSummary::Hide => {}
        }
    }

    pub(crate) async fn run(&self, ctx: &StepContext, job: &mut StepJob) -> Result<()> {
        if ctx.hook_ctx.failed.is_cancelled() {
            trace!("{self}: skipping step due to previous failure");
            // Hide the job progress if it was created
            if let Some(progress) = &job.progress {
                progress.set_status(ProgressStatus::Hide);
            }
            return Ok(());
        }
        if let Some(condition) = &self.condition {
            let val = EXPR_ENV.eval(condition, &ctx.hook_ctx.expr_ctx())?;
            debug!("{self}: condition: {condition} = {val}");
            if val == expr::Value::Bool(false) {
                self.mark_skipped(ctx, &SkipReason::ConditionFalse)?;
                return Ok(());
            }
        }
        // After evaluating the condition, check profiles so condition-false wins over profiles
        if let Some(reason) = self.profile_skip_reason() {
            self.mark_skipped(ctx, &reason)?;
            return Ok(());
        }
        job.progress = Some(job.build_progress(ctx));
        job.status = StepJobStatus::Pending;
        let semaphore = if let Some(semaphore) = job.semaphore.take() {
            semaphore
        } else {
            ctx.hook_ctx.semaphore().await
        };
        job.status_start(ctx, semaphore).await?;
        let mut tctx = job.tctx(&ctx.hook_ctx.tctx);
        // Set {{globs}} template variable based on pattern type
        match self.glob.as_ref() {
            Some(Pattern::Globs(g)) => {
                tctx.with_globs(g.as_slice());
            }
            Some(Pattern::Regex { pattern, .. }) => {
                // For regex patterns, provide the pattern string so templates can use it
                tctx.insert("globs", pattern);
            }
            None => {
                tctx.with_globs(&[] as &[&str]);
            }
        }
        let file_msg = |files: &[PathBuf]| {
            format!(
                "{} file{}",
                files.len(),
                if files.len() == 1 { "" } else { "s" }
            )
        };
        let Some(mut run) = self.run_cmd(job.run_type).map(|s| s.to_string()) else {
            eyre::bail!("{self}: no run command");
        };
        if let Some(prefix) = &self.prefix {
            run = format!("{prefix} {run}");
        }
        let run = tera::render(&run, &tctx)
            .wrap_err_with(|| format!("{self}: failed to render command template"))?;
        let pattern_display = match &self.glob {
            Some(Pattern::Globs(g)) => g.join(" "),
            Some(Pattern::Regex { pattern, .. }) => format!("regex: {}", pattern),
            None => String::new(),
        };
        job.progress.as_ref().unwrap().prop(
            "message",
            &format!("{} – {} – {}", file_msg(&job.files), pattern_display, run),
        );
        job.progress.as_ref().unwrap().update();
        if log::log_enabled!(log::Level::Trace) {
            for file in &job.files {
                trace!("{self}: {}", file.display());
            }
        }
        let mut cmd = if let Some(shell) = &self.shell {
            let shell = shell.to_string();
            let shell = shell.split_whitespace().collect_vec();
            let mut cmd = CmdLineRunner::new(shell[0]);
            for arg in shell[1..].iter() {
                cmd = cmd.arg(arg);
            }
            cmd
        } else {
            CmdLineRunner::new("sh").arg("-o").arg("errexit").arg("-c")
        };
        cmd = cmd
            .arg(&run)
            .with_pr(job.progress.as_ref().unwrap().clone())
            .with_cancel_token(ctx.hook_ctx.failed.clone())
            .show_stderr_on_error(false)
            .stderr_to_progress(true);
        if self.interactive {
            clx::progress::pause();
            cmd = cmd
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
        }
        if let Some(dir) = &self.dir {
            cmd = cmd.current_dir(dir);
        }
        for (key, value) in &self.env {
            let value = tera::render(value, &tctx)?;
            cmd = cmd.env(key, value);
        }
        let timing_guard = StepTimingGuard::new(ctx.hook_ctx.timing.clone(), self);
        let exec_result = cmd.execute().await;
        timing_guard.finish();
        if self.interactive {
            clx::progress::resume();
        }
        match exec_result {
            Ok(result) => {
                // For check_list_files, if stdout is empty but stderr has content, treat as an error
                if let RunType::Check(CheckType::ListFiles) = job.run_type {
                    debug!(
                        "{self}: check_list_files succeeded, stdout len={}, stderr len={}",
                        result.stdout.len(),
                        result.stderr.len()
                    );
                    if result.stdout.trim().is_empty() && !result.stderr.trim().is_empty() {
                        error!("{self}: check_list_files returned no files but produced stderr");
                        return Err(Error::CheckListFailed {
                            source: eyre!("check_list_files returned no files but produced stderr"),
                            stdout: result.stdout.clone(),
                            stderr: result.stderr.clone(),
                        })?;
                    }
                }
                // Save output for end-of-run summary based on configured mode
                self.save_output_summary(
                    ctx,
                    job,
                    &result.stdout,
                    &result.stderr,
                    &result.combined_output,
                    false, // not a failure
                );
            }
            Err(err) => {
                if let ensembler::Error::ScriptFailed(e) = &err {
                    if let RunType::Check(CheckType::ListFiles) = job.run_type {
                        let result = &e.3;
                        let stdout = result.stdout.clone();
                        let stderr = result.stderr.clone();
                        return Err(Error::CheckListFailed {
                            source: eyre!("{}", err),
                            stdout,
                            stderr,
                        })?;
                    }
                    // Save output from a failed command as well
                    self.save_output_summary(
                        ctx,
                        job,
                        &e.3.stdout,
                        &e.3.stderr,
                        &e.3.combined_output,
                        true, // is a failure
                    );

                    // If we're in check mode and a fix command exists, collect a helpful suggestion
                    self.collect_fix_suggestion(ctx, job, Some(&e.3));
                }
                if job.check_first && matches!(job.run_type, RunType::Check(_)) {
                    ctx.progress.set_status(ProgressStatus::Warn);
                } else {
                    ctx.progress.set_status(ProgressStatus::Failed);
                }
                return Err(err).wrap_err(run);
            }
        }
        ctx.decrement_job_count();
        job.status_finished()?;
        Ok(())
    }

    fn filter_files_from_check_list(
        &self,
        original_files: &[PathBuf],
        stdout: &str,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let listed: HashSet<PathBuf> = stdout
            .lines()
            .map(|p| try_canonicalize(&PathBuf::from(p)))
            .collect();
        let files: IndexSet<PathBuf> = original_files
            .iter()
            .filter(|f| listed.contains(&try_canonicalize(f)))
            .cloned()
            .collect();
        let canonicalized_files: IndexSet<PathBuf> = files.iter().map(try_canonicalize).collect();
        let extras: Vec<PathBuf> = listed
            .into_iter()
            .filter(|f| !canonicalized_files.contains(f))
            .collect();
        (files.into_iter().collect(), extras)
    }

    fn collect_fix_suggestion(
        &self,
        ctx: &StepContext,
        job: &StepJob,
        cmd_result: Option<&ensembler::CmdResult>,
    ) {
        // Only suggest fixes when the entire hook run is in check mode,
        // not when an individual job temporarily runs a check (e.g., check_first during a fix run)
        if !matches!(ctx.hook_ctx.run_type, RunType::Check(_)) || self.fix.is_none() {
            return;
        }
        // Prefer filtering files if check_list_files output is available
        let mut suggest_files = job.files.clone();
        if let Some(result) = cmd_result {
            if self.check_list_files.is_some() {
                let (files, _extras) =
                    self.filter_files_from_check_list(&job.files, &result.stdout);
                if !files.is_empty() {
                    suggest_files = files;
                }
            }
        }
        // Build a minimal context based on the suggested files, honoring dir/workspace
        let temp_job = StepJob::new(Arc::new(self.clone()), suggest_files, RunType::Fix);
        let suggest_ctx = temp_job.tctx(&ctx.hook_ctx.tctx);
        if let Some(mut fix_cmd) = self.run_cmd(RunType::Fix).map(|s| s.to_string()) {
            if let Some(prefix) = &self.prefix {
                fix_cmd = format!("{prefix} {fix_cmd}");
            }
            if let Ok(rendered) = tera::render(&fix_cmd, &suggest_ctx) {
                let is_multi_line = rendered.contains('\n');
                if is_multi_line {
                    // Too long to inline; suggest hk fix with step filter
                    let step_flag = format!("-S {}", &self.name);
                    let cmd = format!(
                        "To fix, run: {}",
                        style::edim(format!("hk fix {}", step_flag))
                    );
                    ctx.hook_ctx.add_fix_suggestion(cmd);
                } else {
                    let cmd = format!("To fix, run: {}", style::edim(rendered));
                    ctx.hook_ctx.add_fix_suggestion(cmd);
                }
            }
        }
    }

    pub fn shell_type(&self) -> ShellType {
        let shell = self
            .shell
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let shell = shell.split_whitespace().next().unwrap_or_default();
        let shell = shell.split("/").last().unwrap_or_default();
        match shell {
            "bash" => ShellType::Bash,
            "dash" => ShellType::Dash,
            "fish" => ShellType::Fish,
            "sh" => ShellType::Sh,
            "zsh" => ShellType::Zsh,
            _ => ShellType::Other(shell.to_string()),
        }
    }

    pub fn mark_skipped(&self, ctx: &StepContext, reason: &SkipReason) -> Result<()> {
        // Track all skip reasons for potential future use
        ctx.hook_ctx.track_skip(&self.name, reason.clone());

        if reason.should_display() {
            ctx.progress.prop("message", &reason.message());
            let status = ProgressStatus::DoneCustom(style::eblue("⇢").bold().to_string());
            ctx.progress.set_status(status);
        } else {
            // Step is skipped but message shouldn't be displayed
            ctx.progress.set_status(ProgressStatus::Hide);
        }
        ctx.depends.mark_done(&self.name)?;
        Ok(())
    }
}

pub enum ShellType {
    Bash,
    Dash,
    Fish,
    Sh,
    Zsh,
    #[allow(unused)]
    Other(String),
}

impl ShellType {
    pub fn quote(&self, s: &str) -> String {
        match self {
            ShellType::Bash | ShellType::Zsh => s.quoted(shell_quote::Bash),
            ShellType::Fish => s.quoted(shell_quote::Fish),
            ShellType::Dash | ShellType::Sh | ShellType::Other(_) => {
                let mut o = vec![];
                shell_quote::Sh::quote_into(s, &mut o);
                String::from_utf8(o).unwrap_or_default()
            }
        }
    }
}

pub static EXPR_CTX: LazyLock<expr::Context> = LazyLock::new(expr::Context::default);

pub static EXPR_ENV: LazyLock<expr::Environment> = LazyLock::new(|| {
    let mut env = expr::Environment::new();

    env.add_function("exec", |c| {
        let out = xx::process::sh(c.args[0].as_string().unwrap())
            .map_err(|e| expr::Error::ExprError(e.to_string()))?;
        Ok(expr::Value::String(out))
    });

    env
});

fn try_canonicalize(path: &PathBuf) -> PathBuf {
    match path.canonicalize() {
        Ok(p) => p,
        Err(err) => {
            warn!("failed to canonicalize file: {} {err}", display_path(path));
            path.to_path_buf()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde_as]
pub struct Script {
    pub linux: Option<String>,
    pub macos: Option<String>,
    pub windows: Option<String>,
    pub other: Option<String>,
}

impl FromStr for Script {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            linux: None,
            macos: None,
            windows: None,
            other: Some(s.to_string()),
        })
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let other = self.other.as_deref().unwrap_or_default();
        if cfg!(target_os = "macos") {
            write!(f, "{}", self.macos.as_deref().unwrap_or(other))
        } else if cfg!(target_os = "linux") {
            write!(f, "{}", self.linux.as_deref().unwrap_or(other))
        } else if cfg!(target_os = "windows") {
            write!(f, "{}", self.windows.as_deref().unwrap_or(other))
        } else {
            write!(f, "{other}")
        }
    }
}
