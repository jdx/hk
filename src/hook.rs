use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressOutput, ProgressStatus};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsString,
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex as StdMutex},
};
use tokio::{
    signal,
    sync::{Mutex, OwnedSemaphorePermit, Semaphore},
};
use tokio_util::sync::CancellationToken;

use crate::{
    Result, env,
    file_rw_locks::FileRwLocks,
    git::{Git, GitStatus, StashMethod},
    glob,
    hook_options::HookOptions,
    plan::{ParallelGroup, Plan, PlannedStep, Reason, ReasonKind, StepStatus},
    settings::Settings,
    step::{EXPR_CTX, EXPR_ENV, OutputSummary, RunType, Script, Step},
    step_context::StepContext,
    step_group::{StepGroup, StepGroupContext},
    timings::TimingRecorder,
    ui::style,
    version,
};

#[derive(Debug, Clone, Eq, PartialEq, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum SkipReason {
    #[strum(serialize = "disabled-by-env")]
    DisabledByEnv(String),
    #[strum(serialize = "disabled-by-cli")]
    DisabledByCli(String),
    #[strum(serialize = "disabled-by-config")]
    DisabledByConfig,
    ProfileNotEnabled(Vec<String>),
    ProfileExplicitlyDisabled,
    #[strum(serialize = "no-command-for-run-type")]
    NoCommandForRunType(RunType),
    NoFilesToProcess,
    ConditionFalse,
    #[strum(serialize = "missing-required-env")]
    MissingRequiredEnv(Vec<String>),
}

impl SkipReason {
    pub fn message(&self) -> String {
        match self {
            SkipReason::DisabledByEnv(src) | SkipReason::DisabledByCli(src) => {
                format!("skipped: disabled via {src}")
            }
            SkipReason::DisabledByConfig => "skipped: disabled via skip configuration".to_string(),
            SkipReason::MissingRequiredEnv(envs) => {
                format!(
                    "skipped: missing required environment variable(s): {}",
                    envs.join(", ")
                )
            }
            SkipReason::ProfileNotEnabled(profiles) => {
                if profiles.is_empty() {
                    "skipped: disabled by profile".to_string()
                } else {
                    format!(
                        "skipped: profile{} not enabled ({})",
                        if profiles.len() > 1 { "s" } else { "" },
                        profiles.join(", ")
                    )
                }
            }
            SkipReason::ProfileExplicitlyDisabled => "skipped: disabled by profile".to_string(),
            SkipReason::NoCommandForRunType(_) => "skipped: no command for run type".to_string(),
            SkipReason::NoFilesToProcess => "skipped: no files to process".to_string(),
            SkipReason::ConditionFalse => "skipped: condition is false".to_string(),
        }
    }

    pub fn should_display(&self) -> bool {
        let settings = Settings::get();
        // Use strum's Display trait to get the kebab-case string
        let key = self.to_string();
        settings.display_skip_reasons.contains(&key)
    }
}

#[serde_as]
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct Hook {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub steps: IndexMap<String, StepOrGroup>,
    pub fix: Option<bool>,
    pub stash: Option<StashSetting>,
    pub stage: Option<bool>,
    #[serde(default)]
    pub fail_on_fix: bool,
    #[serde(default)]
    pub env: IndexMap<String, String>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub report: Option<Script>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum StashSetting {
    Method(StashMethod),
    Bool(bool),
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(tag = "_type", rename_all = "snake_case")]
pub enum StepOrGroup {
    Step(Box<Step>),
    Group(Box<StepGroup>),
}

impl StepOrGroup {
    pub fn init(&mut self, name: &str) -> Result<()> {
        match self {
            StepOrGroup::Step(step) => step.init(name)?,
            StepOrGroup::Group(group) => group.init(name)?,
        }
        Ok(())
    }
}

pub struct HookContext {
    pub file_locks: FileRwLocks,
    pub git: Arc<Mutex<Git>>,
    pub groups: Vec<StepGroup>,
    pub tctx: crate::tera::Context,
    pub run_type: RunType,
    semaphore: Arc<Semaphore>,
    pub failed: CancellationToken,
    pub hk_progress: Option<Arc<ProgressJob>>,
    pub step_contexts: std::sync::Mutex<IndexMap<String, Arc<StepContext>>>,
    pub files_in_contention: std::sync::Mutex<HashSet<PathBuf>>,
    total_jobs: std::sync::Mutex<usize>,
    completed_jobs: std::sync::Mutex<usize>,
    expr_ctx: std::sync::Mutex<expr::Context>,
    pub timing: Arc<TimingRecorder>,
    pub skip_steps: IndexMap<String, SkipReason>,
    skipped_steps: std::sync::Mutex<IndexMap<String, SkipReason>>,
    /// Aggregated output per step name (in insertion order)
    pub output_by_step: std::sync::Mutex<IndexMap<String, (OutputSummary, String)>>,
    /// Collected fix suggestions to display at end of run
    pub fix_suggestions: std::sync::Mutex<Vec<String>>,
    pub should_stage: bool,
    /// Untracked files at the start of the hook run, used to avoid staging
    /// pre-existing untracked files that were not created by a fixer.
    pub initial_untracked: BTreeSet<PathBuf>,
}

impl HookContext {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        files: impl IntoIterator<Item = PathBuf>,
        git: Arc<Mutex<Git>>,
        groups: Vec<StepGroup>,
        tctx: crate::tera::Context,
        expr_ctx: expr::Context,
        run_type: RunType,
        hk_progress: Option<Arc<ProgressJob>>,
        skip_steps: IndexMap<String, SkipReason>,
        should_stage: bool,
        initial_untracked: BTreeSet<PathBuf>,
    ) -> Self {
        let settings = Settings::get();
        let expr_ctx = expr_ctx;
        let mut timing = TimingRecorder::new(env::HK_TIMING_JSON.clone());
        // Pre-populate timing metadata once before any jobs start
        for group in &groups {
            for step in group.steps.values() {
                timing.set_step_profiles(&step.name, step.profiles.as_deref());
                timing.set_step_interactive(&step.name, step.interactive);
            }
        }
        Self {
            file_locks: FileRwLocks::new(files),
            git,
            hk_progress,
            total_jobs: StdMutex::new(groups.iter().map(|g| g.steps.len()).sum()),
            completed_jobs: StdMutex::new(0),
            groups,
            tctx,
            run_type,
            step_contexts: StdMutex::new(Default::default()),
            files_in_contention: StdMutex::new(Default::default()),
            semaphore: Arc::new(Semaphore::new(settings.jobs().get())),
            failed: CancellationToken::new(),
            expr_ctx: StdMutex::new(expr_ctx),
            timing: Arc::new(timing),
            skip_steps,
            skipped_steps: StdMutex::new(IndexMap::new()),
            output_by_step: StdMutex::new(IndexMap::new()),
            fix_suggestions: StdMutex::new(Vec::new()),
            should_stage,
            initial_untracked,
        }
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.file_locks.files()
    }

    pub fn add_files(&self, added_paths: &[PathBuf], created_paths: &[PathBuf]) {
        self.file_locks.add_files(added_paths);
        self.file_locks.add_files(created_paths);
        // self.expr_ctx
        //     .lock()
        //     .unwrap()
        //     .insert("files", expr::to_value(&files).unwrap());
    }

    pub async fn semaphore(&self) -> OwnedSemaphorePermit {
        if let Some(permit) = self.try_semaphore() {
            permit
        } else {
            self.semaphore.clone().acquire_owned().await.unwrap()
        }
    }

    pub fn expr_ctx(&self) -> expr::Context {
        self.expr_ctx.lock().unwrap().clone()
    }

    pub fn try_semaphore(&self) -> Option<OwnedSemaphorePermit> {
        self.semaphore.clone().try_acquire_owned().ok()
    }

    pub fn inc_total_jobs(&self, n: usize) {
        if n > 0 {
            let mut total_jobs = self.total_jobs.lock().unwrap();
            *total_jobs += n;
            let total_jobs = *total_jobs;
            if let Some(hk_progress) = &self.hk_progress {
                hk_progress.progress_total(total_jobs);
            }
        }
    }

    pub fn inc_completed_jobs(&self, n: usize) {
        if n > 0 {
            let mut completed_jobs = self.completed_jobs.lock().unwrap();
            *completed_jobs += n;
            let completed_jobs = *completed_jobs;
            if let Some(hk_progress) = &self.hk_progress {
                hk_progress.progress_current(completed_jobs);
            }
        }
    }

    pub fn dec_total_jobs(&self, n: usize) {
        if n > 0 {
            let mut total_jobs = self.total_jobs.lock().unwrap();
            *total_jobs = total_jobs.saturating_sub(n);
            let total_jobs = *total_jobs;
            if let Some(hk_progress) = &self.hk_progress {
                hk_progress.progress_total(total_jobs);
            }
        }
    }

    pub fn track_skip(&self, step_name: &str, reason: SkipReason) {
        self.skipped_steps
            .lock()
            .unwrap()
            .insert(step_name.to_string(), reason);
    }

    pub fn get_skipped_steps(&self) -> IndexMap<String, SkipReason> {
        self.skipped_steps.lock().unwrap().clone()
    }

    pub fn append_step_output(&self, step_name: &str, mode: OutputSummary, text: &str) {
        if text.is_empty() {
            return;
        }
        let mut map = self.output_by_step.lock().unwrap();
        map.entry(step_name.to_string())
            .and_modify(|(_, s)| s.push_str(text))
            .or_insert_with(|| (mode, text.to_string()));
    }

    pub fn add_fix_suggestion(&self, suggestion: String) {
        self.fix_suggestions.lock().unwrap().push(suggestion);
    }

    pub fn take_fix_suggestions(&self) -> Vec<String> {
        self.fix_suggestions.lock().unwrap().clone()
    }
}

impl Hook {
    pub fn init(&mut self, hook_name: &str) -> Result<()> {
        self.name = hook_name.to_string();
        for (name, step_or_group) in self.steps.iter_mut() {
            step_or_group.init(name)?;
            // Merge hook-level env into steps (step-level env takes precedence)
            if !self.env.is_empty() {
                match step_or_group {
                    StepOrGroup::Step(step) => {
                        for (key, value) in &self.env {
                            step.env.entry(key.clone()).or_insert_with(|| value.clone());
                        }
                    }
                    StepOrGroup::Group(group) => {
                        for step in group.steps.values_mut() {
                            for (key, value) in &self.env {
                                step.env.entry(key.clone()).or_insert_with(|| value.clone());
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn run_type(&self, opts: &HookOptions) -> RunType {
        let fix = self.fix.unwrap_or(self.name == "fix");
        if (*env::HK_FIX && fix) || opts.fix {
            RunType::Fix
        } else {
            RunType::Check
        }
    }

    fn get_step_groups(&self, opts: &HookOptions) -> Vec<StepGroup> {
        let mut steps = self.steps.values().cloned().collect_vec();
        if !opts.step.is_empty() {
            steps = steps
                .into_iter()
                .filter_map(|s| match s {
                    StepOrGroup::Step(ref step) => opts.step.contains(&step.name).then_some(s),
                    StepOrGroup::Group(mut group) => {
                        group.steps.retain(|s, _| opts.step.contains(s));
                        Some(StepOrGroup::Group(group))
                    }
                })
                .collect_vec();
        }
        StepGroup::build_all(steps)
    }

    fn resolve_stash_method(&self, env_stash: Option<StashMethod>) -> StashMethod {
        if let Some(env_val) = env_stash {
            return env_val;
        }
        match &self.stash {
            Some(StashSetting::Method(m)) => *m,
            Some(StashSetting::Bool(b)) => {
                if *b {
                    StashMethod::Git
                } else {
                    StashMethod::None
                }
            }
            None => StashMethod::None,
        }
    }

    pub async fn plan(&self, opts: HookOptions) -> Result<()> {
        // Suppress progress output so plan output (especially JSON) is clean.
        clx::progress::set_output(ProgressOutput::Text);
        let settings = Settings::get();
        let run_type = self.run_type(&opts);
        let repo = Arc::new(Mutex::new(Git::new()?));
        let git_status = repo.lock().await.status(None)?;
        let stash_method = if let Some(stash_str) = &opts.stash {
            stash_str
                .parse::<StashMethod>()
                .unwrap_or(StashMethod::None)
        } else {
            self.resolve_stash_method(*env::HK_STASH)
        };
        let progress = ProgressJobBuilder::new()
            .status(ProgressStatus::Hide)
            .build();
        let files: Vec<PathBuf> = self
            .file_list(&opts, repo.clone(), &git_status, stash_method, &progress)
            .await?
            .into_iter()
            .collect();

        let skip_steps = build_skip_steps(&settings, &opts);

        let groups = self.get_step_groups(&opts);
        let expr_ctx = build_expr_ctx(&git_status);

        let mut plan = Plan::new(self.name.clone(), run_type.as_str().to_string())
            .with_profiles(settings.enabled_profiles().iter().cloned().collect());

        let mut order_index: usize = 0;
        for (group_idx, group) in groups.iter().enumerate() {
            let group_id = format!("group_{}", group_idx);
            let multi = group.steps.len() > 1;
            let mut group_step_ids: Vec<String> = Vec::new();

            for (step_name, step) in &group.steps {
                let (status, reasons, file_count) =
                    self.analyze_step(step, &files, run_type, &skip_steps, &expr_ctx, &opts);

                let planned = PlannedStep {
                    name: step_name.clone(),
                    status,
                    order_index,
                    parallel_group_id: if multi { Some(group_id.clone()) } else { None },
                    depends_on: step.depends.clone(),
                    reasons,
                    file_count,
                    metadata: HashMap::new(),
                };

                plan.add_step(planned);
                group_step_ids.push(step_name.clone());
                order_index += 1;
            }

            if multi {
                plan.add_group(ParallelGroup {
                    id: group_id,
                    step_ids: group_step_ids,
                });
            }
        }

        if opts.json {
            let json = serde_json::to_string_pretty(&plan)?;
            println!("{}", json);
        } else {
            self.print_plan(&plan, &opts);
        }
        Ok(())
    }

    fn analyze_step(
        &self,
        step: &Step,
        files: &[PathBuf],
        run_type: RunType,
        skip_steps: &IndexMap<String, SkipReason>,
        expr_ctx: &expr::Context,
        opts: &HookOptions,
    ) -> (StepStatus, Vec<Reason>, Option<usize>) {
        let mut reasons: Vec<Reason> = Vec::new();

        // Mirror the runtime: step_condition is evaluated in execution.rs before
        // build_step_jobs and can skip the step entirely. Only a literal
        // Bool(false) causes a skip — any other value (including non-bool
        // results from exec()) is truthy.
        if let Some(condition) = &step.step_condition {
            match EXPR_ENV.eval(condition, expr_ctx) {
                Ok(val) if val == expr::Value::Bool(false) => {
                    reasons.push(Reason {
                        kind: ReasonKind::ConditionFalse,
                        detail: Some(format!("step_condition evaluated to false: {}", condition)),
                        data: HashMap::new(),
                    });
                    return (StepStatus::Skipped, reasons, None);
                }
                Ok(_) => {
                    reasons.push(Reason {
                        kind: ReasonKind::ConditionTrue,
                        detail: Some(format!("step_condition evaluated to true: {}", condition)),
                        data: HashMap::new(),
                    });
                }
                Err(err) => {
                    reasons.push(Reason {
                        kind: ReasonKind::ConditionUnknown,
                        detail: Some(format!("step_condition could not be evaluated: {err}")),
                        data: HashMap::new(),
                    });
                }
            }
        }

        // Let build_step_jobs do the heavy lifting for skip detection.
        let jobs = match step.build_step_jobs(files, run_type, &Default::default(), skip_steps) {
            Ok(j) => j,
            Err(err) => {
                reasons.push(Reason {
                    kind: ReasonKind::Disabled,
                    detail: Some(format!("failed to plan step: {err}")),
                    data: HashMap::new(),
                });
                return (StepStatus::Skipped, reasons, None);
            }
        };

        let all_skipped = !jobs.is_empty() && jobs.iter().all(|j| j.skip_reason.is_some());

        if all_skipped {
            if let Some(reason) = jobs.iter().find_map(|j| j.skip_reason.as_ref()) {
                reasons.push(skip_reason_to_reason(reason));
            } else {
                reasons.push(Reason {
                    kind: ReasonKind::Disabled,
                    detail: None,
                    data: HashMap::new(),
                });
            }
            return (StepStatus::Skipped, reasons, Some(0));
        }

        let file_count: usize = jobs.iter().map(|j| j.files.len()).sum();

        // Mirror runner.rs: job_condition is evaluated at run-time, and only a
        // literal Bool(false) skips the step. Truthy values (including strings)
        // pass through.
        if let Some(condition) = &step.job_condition {
            match EXPR_ENV.eval(condition, expr_ctx) {
                Ok(val) if val == expr::Value::Bool(false) => {
                    reasons.push(Reason {
                        kind: ReasonKind::ConditionFalse,
                        detail: Some(format!("condition evaluated to false: {}", condition)),
                        data: HashMap::new(),
                    });
                    return (StepStatus::Skipped, reasons, Some(file_count));
                }
                Ok(_) => {
                    reasons.push(Reason {
                        kind: ReasonKind::ConditionTrue,
                        detail: Some(format!("condition evaluated to true: {}", condition)),
                        data: HashMap::new(),
                    });
                }
                Err(err) => {
                    reasons.push(Reason {
                        kind: ReasonKind::ConditionUnknown,
                        detail: Some(format!("condition could not be evaluated: {err}")),
                        data: HashMap::new(),
                    });
                }
            }
        }

        // build_step_jobs defers profile checks when job_condition is set
        // (see job_builder.rs) — the runtime evaluates them after the condition
        // in runner.rs. Mirror that here so a profile-skipped step isn't
        // reported as Included.
        if step.job_condition.is_some()
            && let Some(reason) = step.profile_skip_reason()
        {
            reasons.push(skip_reason_to_reason(&reason));
            return (StepStatus::Skipped, reasons, Some(file_count));
        }

        // Files matched
        reasons.push(Reason {
            kind: ReasonKind::FilterMatch,
            detail: Some(format!(
                "{} file{} matched",
                file_count,
                if file_count == 1 { "" } else { "s" }
            )),
            data: HashMap::new(),
        });

        // Profile include (only meaningful when profiles are configured)
        if step.profiles.is_some() && step.profile_skip_reason().is_none() {
            reasons.push(Reason {
                kind: ReasonKind::ProfileInclude,
                detail: Some("required profile(s) enabled".to_string()),
                data: HashMap::new(),
            });
        }

        // CLI --step inclusion
        if !opts.step.is_empty() && opts.step.contains(&step.name) {
            reasons.push(Reason {
                kind: ReasonKind::CliInclude,
                detail: Some(format!("explicitly included via --step {}", step.name)),
                data: HashMap::new(),
            });
        }

        (StepStatus::Included, reasons, Some(file_count))
    }

    fn print_plan(&self, plan: &Plan, opts: &HookOptions) {
        println!("{} {}", style::nbold("Plan:"), style::ncyan(&plan.hook));
        println!("{} {}", style::ndim("Run type:"), plan.run_type);
        if !plan.profiles.is_empty() {
            println!("{} {}", style::ndim("Profiles:"), plan.profiles.join(", "));
        }
        println!();

        // --why <step> focuses output on one step; --why alone shows all reasons for all steps.
        let (focus_step, verbose) = match opts.why.as_deref() {
            None => (None, false),
            Some("") => (None, true),
            Some(s) => (Some(s.to_string()), true),
        };

        let mut last_group: Option<String> = None;
        for step in &plan.steps {
            if let Some(focus) = &focus_step
                && focus != &step.name
            {
                continue;
            }
            if step.parallel_group_id != last_group
                && let Some(gid) = &step.parallel_group_id
            {
                println!("  {} {}", style::ndim("[parallel group]"), style::ndim(gid));
            }
            last_group = step.parallel_group_id.clone();

            let (icon, name_style) = if step.status == StepStatus::Included {
                (
                    style::ncyan("✓").to_string(),
                    style::ncyan(&step.name).to_string(),
                )
            } else {
                (
                    style::ndim("○").to_string(),
                    style::ndim(&step.name).to_string(),
                )
            };

            let headline = step
                .reasons
                .first()
                .map(|r| {
                    r.detail
                        .clone()
                        .unwrap_or_else(|| r.kind.short_description().to_string())
                })
                .unwrap_or_default();

            let indent = if step.parallel_group_id.is_some() {
                "    "
            } else {
                "  "
            };
            println!(
                "{indent}{icon} {name_style}  {}",
                style::ndim(format!("({headline})"))
            );

            if verbose {
                for reason in step.reasons.iter().skip(1) {
                    let detail = reason
                        .detail
                        .clone()
                        .unwrap_or_else(|| reason.kind.short_description().to_string());
                    println!("{indent}    - {detail}");
                }
                if !step.depends_on.is_empty() {
                    println!(
                        "{indent}    - {} {}",
                        style::ndim("depends on:"),
                        step.depends_on.join(", ")
                    );
                }
            }
        }
    }

    pub async fn stats(&self, opts: HookOptions, hook_name: &str) -> Result<()> {
        let settings = Settings::get();
        let run_type = self.run_type(&opts);
        let repo = Arc::new(Mutex::new(Git::new()?));
        let git_status = repo.lock().await.status(None)?;
        let stash_method = if let Some(stash_str) = &opts.stash {
            stash_str
                .parse::<StashMethod>()
                .unwrap_or(StashMethod::None)
        } else {
            self.resolve_stash_method(*env::HK_STASH)
        };
        let progress = ProgressJobBuilder::new()
            .status(ProgressStatus::Hide)
            .build();
        let files = self
            .file_list(&opts, repo.clone(), &git_status, stash_method, &progress)
            .await?;
        let all_files = files.iter().cloned().collect::<Vec<_>>();
        let total_files = all_files.len();

        let skip_steps = build_skip_steps(&settings, &opts);

        println!(
            "{}",
            style::nbold(&format!("Statistics for hook: {}", hook_name))
        );
        println!();
        println!("Total files: {}", style::ncyan(total_files));
        println!("Run type: {}", style::nblue(run_type.as_str()));
        println!();

        // Collect stats for each step
        let groups = self.get_step_groups(&opts);
        let mut step_stats: Vec<(String, usize, Option<SkipReason>)> = Vec::new();

        for group in &groups {
            for (step_name, step) in &group.steps {
                // Check if step is in skip list
                if let Some(skip_reason) = skip_steps.get(step_name) {
                    step_stats.push((step_name.clone(), 0, Some(skip_reason.clone())));
                    continue;
                }

                // Check if step has a command for this run type
                if !step.has_command_for(run_type) {
                    step_stats.push((
                        step_name.clone(),
                        0,
                        Some(SkipReason::NoCommandForRunType(run_type)),
                    ));
                    continue;
                }

                // Check profile skip reason
                if let Some(skip_reason) = step.profile_skip_reason() {
                    step_stats.push((step_name.clone(), 0, Some(skip_reason)));
                    continue;
                }

                let filtered_files = step.filter_files(&all_files)?;
                step_stats.push((step_name.clone(), filtered_files.len(), None));
            }
        }

        if step_stats.is_empty() {
            println!("No steps found in hook '{}'", hook_name);
            return Ok(());
        }

        println!("{}", style::nbold("Files matching each step:"));
        println!();

        // Find the longest step name for alignment
        let max_name_len = step_stats
            .iter()
            .map(|(name, _, _)| name.len())
            .max()
            .unwrap_or(0);

        for (step_name, count, skip_reason) in step_stats {
            if let Some(reason) = skip_reason {
                println!(
                    "  {:width$}  {}",
                    style::nyellow(&step_name),
                    style::ndim(format!("(skipped: {})", reason.message())),
                    width = max_name_len
                );
            } else {
                let percentage = if total_files > 0 {
                    (count as f64 / total_files as f64) * 100.0
                } else {
                    0.0
                };

                println!(
                    "  {:width$}  {}  ({:.1}%)",
                    style::nyellow(&step_name),
                    style::ncyan(count),
                    percentage,
                    width = max_name_len
                );
            }
        }

        Ok(())
    }

    #[tracing::instrument(level = "info", name = "hook.run", skip(self, opts), fields(hook = %self.name))]
    pub async fn run(&self, opts: HookOptions) -> Result<()> {
        tracing::info!("running hook");
        let settings = Settings::get();
        let fail_fast = if opts.fail_fast {
            true
        } else if opts.no_fail_fast {
            false
        } else {
            settings.fail_fast
        };
        let should_stage = opts
            .should_stage()
            .or(settings.stage)
            .or(self.stage)
            .unwrap_or(true);

        if settings.skip_hooks.contains(&self.name) {
            warn!("{}: skipping hook due to HK_SKIP_HOOK", &self.name);
            return Ok(());
        }
        let run_type = self.run_type(&opts);
        let repo = Arc::new(Mutex::new(Git::new()?));
        let groups = self.get_step_groups(&opts);
        let stash_method = if let Some(stash_str) = &opts.stash {
            stash_str
                .parse::<StashMethod>()
                .unwrap_or(StashMethod::None)
        } else {
            self.resolve_stash_method(*env::HK_STASH)
        };
        let total_steps: usize = groups.iter().map(|g| g.steps.len()).sum();
        let hk_progress = self.start_hk_progress(run_type, total_steps);
        let file_progress = ProgressJobBuilder::new().body(
            "{{spinner()}} files - {{message}}{% if files is defined %} ({{files}} file{{files|pluralize}}){% endif %}"
        )
        .prop("message", "Fetching git status")
        .start();
        let git_status = repo.lock().await.status(None)?;
        let files = self
            .file_list(
                &opts,
                repo.clone(),
                &git_status,
                stash_method,
                &file_progress,
            )
            .await?;

        let skip_steps = build_skip_steps(&settings, &opts);
        if files.is_empty() && can_exit_early(&groups, &files, run_type, &skip_steps) {
            info!("no files to run");
            if let Some(hk_progress) = &hk_progress {
                hk_progress.set_status(ProgressStatus::Hide);
            }
            return Ok(());
        }
        // Enrich Tera and expr contexts with git status for template/condition use
        let git_status_for_ctx = git_status.clone();
        let mut tctx = opts.tctx;
        // Insert a serializable view under "git"
        tctx.insert("git", &git_status_for_ctx);
        tctx.insert("hook", &self.name);
        let expr_ctx = build_expr_ctx(&git_status_for_ctx);
        let hook_ctx = Arc::new(HookContext::new(
            files,
            repo.clone(),
            groups,
            tctx,
            expr_ctx,
            run_type,
            hk_progress,
            skip_steps,
            should_stage,
            git_status.untracked_files.clone(),
        ));

        watch_for_ctrl_c(hook_ctx.failed.clone());

        if stash_method != StashMethod::None {
            // Only run stash logic if there are actually unstaged changes to stash
            let has_unstaged_changes = !git_status.unstaged_files.is_empty()
                || (*env::HK_STASH_UNTRACKED && !git_status.untracked_files.is_empty());

            if has_unstaged_changes {
                // Capture exact staged index entries for files under consideration so we can
                // ensure index hunks survive formatting and stash apply.
                let files_vec = hook_ctx.files();
                {
                    let mut r = repo.lock().await;
                    r.capture_index(&files_vec)?;
                    // Stash ALL unstaged changes in the repository (not only files under consideration)
                    // so that unrelated worktree changes do not affect or get affected by fixers.
                    r.stash_unstaged(&file_progress, stash_method, &git_status)?;
                }
            } else {
                file_progress.prop("message", "No unstaged changes to stash");
                file_progress.set_status(ProgressStatus::Done);
            }
        }

        if hook_ctx.groups.is_empty() {
            info!("no steps to run");
            return Ok(());
        }
        // Snapshot file content hashes before running groups so fail_on_fix can detect
        // which files were actually modified by fixers (ignoring pre-existing changes).
        let pre_file_hashes: std::collections::HashMap<PathBuf, u64> =
            if self.fail_on_fix && matches!(run_type, RunType::Fix) {
                use std::hash::{Hash, Hasher};
                hook_ctx
                    .files()
                    .into_iter()
                    .filter_map(|f| {
                        std::fs::read(&f).ok().map(|content| {
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            content.hash(&mut hasher);
                            (f, hasher.finish())
                        })
                    })
                    .collect()
            } else {
                std::collections::HashMap::new()
            };

        let mut result = Ok(());
        let multiple_groups = hook_ctx.groups.len() > 1;
        for (i, group) in hook_ctx.groups.iter().enumerate() {
            debug!("running group: {i}");
            let mut ctx = StepGroupContext::new(hook_ctx.clone(), fail_fast);
            if multiple_groups && let Some(name) = &group.name {
                ctx = ctx.with_progress(group.build_group_progress(name));
            }
            result = result.and(group.run(ctx).await);
            if fail_fast && result.is_err() {
                break;
            }
        }
        // When fail_on_fix is enabled, fail if fix commands actually modified files.
        // Compares file content hashes against pre-run snapshot.
        if result.is_ok() && self.fail_on_fix && matches!(run_type, RunType::Fix) {
            use std::hash::{Hash, Hasher};
            let modified_files: Vec<_> = pre_file_hashes
                .iter()
                .filter(|(path, pre_hash)| {
                    std::fs::read(path)
                        .ok()
                        .map(|content| {
                            let mut hasher = std::collections::hash_map::DefaultHasher::new();
                            content.hash(&mut hasher);
                            hasher.finish() != **pre_hash
                        })
                        .unwrap_or(true) // file was deleted by fixer
                })
                .map(|(path, _)| path)
                .collect();
            if !modified_files.is_empty() {
                let file_list = modified_files
                    .iter()
                    .map(|p| format!("  {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                warn!(
                    "Files were modified by fix commands (fail_on_fix=true):\n{}",
                    file_list
                );
                result = Err(eyre::eyre!(
                    "fail_on_fix: files were modified by fix commands"
                ));
            }
        }

        if let Some(hk_progress) = hook_ctx.hk_progress.as_ref() {
            if result.is_ok() {
                hk_progress.set_status(ProgressStatus::Done);
            } else {
                hk_progress.set_status(ProgressStatus::Failed);
            }
        }

        if let Err(err) = repo.lock().await.pop_stash() {
            if result.is_ok() {
                result = Err(err);
            } else {
                warn!("Failed to pop stash: {err}");
            }
        }
        // Capture final git state for diagnostics (counts at debug, names at trace)
        match repo.lock().await.status(None) {
            Ok(s) => {
                debug!(
                    "final git state: staged={} unstaged={}",
                    s.staged_files.len(),
                    s.unstaged_files.len()
                );
                trace!(
                    "final git files: staged={:?} unstaged={:?}",
                    s.staged_files, s.unstaged_files
                );
            }
            Err(e) => warn!("failed to read final git status: {e:?}"),
        }
        if let Err(err) = hook_ctx.timing.write_json() {
            warn!("Failed to write timing JSON: {err}");
        }

        // Clear progress bars before displaying summary
        clx::progress::stop();

        // Display aggregated output from steps, once per step
        if clx::progress::output() != ProgressOutput::Text || *env::HK_SUMMARY_TEXT {
            let outputs = hook_ctx.output_by_step.lock().unwrap().clone();
            for (step_name, (mode, output)) in outputs.into_iter() {
                let trimmed = output.trim_end();
                if trimmed.is_empty() {
                    continue;
                }
                let label = match mode {
                    OutputSummary::Stdout => "stdout",
                    OutputSummary::Stderr => "stderr",
                    OutputSummary::Combined => "output",
                    OutputSummary::Hide => continue,
                };
                eprintln!("\n{}", style::ebold(format!("{} {}:", step_name, label)));
                eprintln!("{}", trimmed);
            }
        }

        // Display summary of profile-skipped steps
        // Only show summary if user has enabled the warning tag and it's not hidden
        if settings.warnings.contains("missing-profiles")
            && !settings.hide_warnings.contains("missing-profiles")
        {
            let skipped_steps = hook_ctx.get_skipped_steps();
            let mut profile_skipped: Vec<String> = vec![];
            let mut missing_profiles = indexmap::IndexSet::new();

            for (name, reason) in skipped_steps.iter() {
                if let SkipReason::ProfileNotEnabled(profiles) = reason {
                    profile_skipped.push(name.clone());
                    missing_profiles.extend(profiles.clone());
                }
            }

            if !profile_skipped.is_empty() {
                let count = profile_skipped.len();
                let profiles_list = missing_profiles.iter().join(", ");
                warn!(
                    "{count} {} skipped due to missing profiles: {profiles_list}",
                    if count == 1 { "step was" } else { "steps were" },
                );

                // Show appropriate help message based on hook type
                let (hk_profile_env, hk_profile_flag) = if missing_profiles.contains("slow") {
                    ("HK_PROFILE=slow".to_string(), "--slow".to_string())
                } else {
                    let default = "slow".to_string();
                    let profile = missing_profiles.iter().next().unwrap_or(&default);
                    (
                        format!("HK_PROFILE={profile}"),
                        format!("--profile={profile}"),
                    )
                };
                let hk_profile_env = style::edim(hk_profile_env);
                if self.name == "pre-commit" || self.name == "pre-push" {
                    let default_branch = repo.lock().await.resolve_default_branch();
                    let hk_fix_cmd =
                        format!("hk fix {hk_profile_flag} --from-ref={default_branch}");
                    let hk_fix_cmd = style::edim(hk_fix_cmd);
                    warn!(
                        "  To enable these steps, set {hk_profile_env} environment variable or run {hk_fix_cmd}"
                    );
                } else {
                    let hk_profile_flag = style::edim(hk_profile_flag);
                    warn!(
                        "  To enable these steps, use {hk_profile_flag} or set {hk_profile_env}."
                    );
                }
                let hide_warning_env = style::edim("HK_HIDE_WARNINGS=missing-profiles");
                warn!("  To hide this warning: set {hide_warning_env}");
                let hide_warning_pkl = style::edim(r#"hide_warnings = List("missing-profiles")"#);
                let config_path = env::HK_CONFIG_DIR.join("config.pkl");
                warn!("  or set {hide_warning_pkl} in {}", config_path.display());
            }
        }

        // Run hook-level report if configured
        if let Some(report) = &self.report
            && let Ok(json) = hook_ctx.timing.to_json_string()
        {
            let mut cmd = ensembler::CmdLineRunner::new("sh")
                .arg("-o")
                .arg("errexit")
                .arg("-c");
            let run = report.to_string();
            cmd = cmd.arg(&run).env("HK_REPORT_JSON", json);
            let pr = ProgressJobBuilder::new()
                .body("report: {{message}}")
                .prop("message", &run)
                .start();
            cmd = cmd.with_pr(pr);
            if let Err(err) = cmd.execute().await {
                warn!("Report command failed: {err}");
            }
        }
        // Emit collected fix suggestions at the end (after progress bars and summaries)
        let suggestions = hook_ctx.take_fix_suggestions();
        if !suggestions.is_empty() {
            for s in suggestions {
                error!("{}", s);
            }
        }
        if let Err(err) = &result {
            // ScriptFailed errors are displayed via output_by_step above, skip logging here
            // Other errors are unexpected, show full trace for debugging
            let is_script_failed = err.chain().any(|e| {
                matches!(
                    e.downcast_ref::<ensembler::Error>(),
                    Some(ensembler::Error::ScriptFailed(_))
                )
            });
            if !is_script_failed {
                error!("{self}: hook finished with error: {err:?}");
            }
        } else {
            debug!("{self}: hook finished successfully");
        }
        result
    }

    async fn file_list(
        &self,
        opts: &HookOptions,
        repo: Arc<Mutex<Git>>,
        git_status: &GitStatus,
        stash_method: StashMethod,
        file_progress: &ProgressJob,
    ) -> Result<BTreeSet<PathBuf>> {
        const EMPTY_REF: &str = "0000000000000000000000000000000000000000";
        let stash = stash_method != StashMethod::None;
        let mut files = if let Some(files) = &opts.files {
            files
                .iter()
                .map(|f| {
                    let p = PathBuf::from(f);
                    if p.is_dir() {
                        all_files_in_dir(&p)
                    } else {
                        Ok(vec![p])
                    }
                })
                .flatten_ok()
                .collect::<Result<BTreeSet<_>>>()?
        } else if let Some(glob) = &opts.glob {
            file_progress.prop("message", "Fetching files matching glob");
            let pathspec = glob.iter().map(OsString::from).collect::<Vec<_>>();
            let mut all_files = repo.lock().await.all_files(Some(&pathspec))?;
            if !stash {
                all_files.extend(git_status.untracked_files.iter().cloned());
            }
            let all_files = all_files.into_iter().collect_vec();
            glob::get_matches(glob, &all_files)?.into_iter().collect()
        } else if let Some(from) = &opts.from_ref {
            if opts.to_ref.as_deref() == Some(EMPTY_REF) {
                file_progress.prop("message", "No files to compare for remote branch deletion");
                BTreeSet::new()
            } else {
                file_progress.prop(
                    "message",
                    &if let Some(to) = &opts.to_ref {
                        format!("Fetching files between {from} and {to}")
                    } else {
                        format!("Fetching files changed since {from}")
                    },
                );
                repo.lock()
                    .await
                    .files_between_refs(from, opts.to_ref.as_deref())?
                    .into_iter()
                    .collect()
            }
        } else if opts.all {
            file_progress.prop("message", "Fetching all files in repo");
            let mut all_files = repo.lock().await.all_files(None)?;
            if !stash {
                all_files.extend(git_status.untracked_files.iter().cloned());
            }
            all_files
        } else if stash {
            file_progress.prop("message", "Fetching staged files");
            git_status.staged_files.iter().cloned().collect()
        } else {
            file_progress.prop("message", "Fetching modified files");
            git_status
                .staged_files
                .iter()
                .chain(git_status.unstaged_files.iter())
                .chain(git_status.untracked_files.iter())
                .cloned()
                .collect()
        };

        // Strip leading "./" from all paths for consistent matching
        files = files
            .into_iter()
            .map(|f| {
                f.to_str()
                    .and_then(|s| s.strip_prefix("./"))
                    .map(PathBuf::from)
                    .unwrap_or(f)
            })
            .collect();

        // Filter out directories (including symlinks to directories)
        // git ls-files includes symlinks, which may point to directories
        files.retain(|f| {
            // First check if it's a symlink using symlink_metadata (doesn't follow links)
            if let Ok(symlink_meta) = std::fs::symlink_metadata(f) {
                if symlink_meta.is_symlink() {
                    // For symlinks, follow them to check if they point to directories
                    if let Ok(metadata) = std::fs::metadata(f) {
                        // Symlink points to something - keep only if not a directory
                        !metadata.is_dir()
                    } else {
                        // Broken symlink (metadata() fails) - keep it
                        // Some tools like check-symlinks need to detect broken symlinks
                        true
                    }
                } else {
                    // Not a symlink - keep if not a directory
                    !symlink_meta.is_dir()
                }
            } else {
                // Can't stat it at all (deleted/renamed) - keep it
                true
            }
        });

        // Union excludes from Settings and CLI options
        let settings = crate::settings::Settings::get();
        let mut all_excludes = settings.exclude.clone();

        // Add CLI --exclude patterns
        if let Some(cli_excludes) = &opts.exclude {
            all_excludes.extend(cli_excludes.iter().cloned());
        }

        if !all_excludes.is_empty() {
            // Process excludes - handle both directory patterns and glob patterns
            debug!(
                "files.exclude: patterns from settings/CLI: {:?}",
                &all_excludes
            );
            let files_before = files.len();
            let mut expanded_excludes = Vec::new();
            for exclude in &all_excludes {
                expanded_excludes.push(exclude.clone());
                // If the pattern doesn't contain glob characters, also add patterns for directory contents
                if !exclude.contains('*') && !exclude.contains('?') && !exclude.contains('[') {
                    expanded_excludes.push(format!("{}/*", exclude));
                    expanded_excludes.push(format!("{}/**", exclude));
                }
            }
            debug!("files.exclude: expanded patterns: {:?}", &expanded_excludes);

            let f = files.iter().collect::<Vec<_>>();
            let exclude_files = glob::get_matches(&expanded_excludes, &f)?
                .into_iter()
                .collect::<HashSet<_>>();
            debug!(
                "files.exclude: matched and will exclude {} file(s)",
                exclude_files.len()
            );
            files.retain(|f| !exclude_files.contains(f));
            debug!(
                "files.exclude: filtered files from {} to {}",
                files_before,
                files.len()
            );
        }
        file_progress.prop("files", &files.len());
        file_progress.set_status(ProgressStatus::Done);
        debug!("files: {files:?}");
        Ok(files)
    }

    fn start_hk_progress(&self, run_type: RunType, total_jobs: usize) -> Option<Arc<ProgressJob>> {
        if clx::progress::output() == ProgressOutput::Text {
            return None;
        }
        let mut hk_progress = ProgressJobBuilder::new()
            .body("{{hk}}{{hook}}{{message}}  {{progress_bar(flex=true)}} {{cur}}/{{total}}")
            .body_text(Some("{{hk}}{{hook}}{{message}}"))
            .prop(
                "hk",
                &format!(
                    "{} {} {}",
                    style::emagenta("hk").bold(),
                    style::edim(version::version()),
                    style::edim("by @jdx")
                )
                .to_string(),
            )
            .progress_current(0)
            .progress_total(total_jobs);
        if self.name == "check" || self.name == "fix" {
            hk_progress = hk_progress.prop("hook", "");
        } else {
            hk_progress = hk_progress.prop(
                "hook",
                &style::edim(format!(" – {}", self.name)).to_string(),
            );
        }
        if run_type == RunType::Fix {
            hk_progress = hk_progress.prop("message", &style::edim(" – fix").to_string());
        } else {
            hk_progress = hk_progress.prop("message", &style::edim(" – check").to_string());
        }
        Some(hk_progress.start())
    }
}

impl fmt::Display for Hook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

fn watch_for_ctrl_c(cancel: CancellationToken) {
    tokio::spawn(async move {
        if let Err(err) = signal::ctrl_c().await {
            warn!("Failed to watch for ctrl-c: {err}");
        }
        tokio::spawn(async move {
            // exit immediately on second ctrl-c
            signal::ctrl_c().await.unwrap();
            std::process::exit(1);
        });
        cancel.cancel();
    });
}

fn all_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    if Settings::get().walk_ignore {
        let walker = ignore::WalkBuilder::new(dir)
            .hidden(false) // Allow dotfiles like .gitignore, .env, etc.
            .build();
        for result in walker {
            let entry = result?;
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                files.push(entry.into_path());
            }
        }
    } else {
        for entry in xx::file::ls(dir)? {
            if entry.is_dir() {
                files.extend(all_files_in_dir(&entry)?);
            } else {
                files.push(entry);
            }
        }
    }
    Ok(files)
}

fn build_skip_steps(settings: &Settings, opts: &HookOptions) -> IndexMap<String, SkipReason> {
    let mut m: IndexMap<String, SkipReason> = IndexMap::new();
    for s in env::HK_SKIP_STEPS.iter() {
        m.insert(
            s.clone(),
            SkipReason::DisabledByEnv("HK_SKIP_STEPS".to_string()),
        );
    }
    for s in settings.skip_steps.iter() {
        if !m.contains_key::<str>(s) {
            m.insert(s.clone(), SkipReason::DisabledByConfig);
        }
    }
    for s in opts.skip_step.iter() {
        m.insert(
            s.clone(),
            SkipReason::DisabledByCli(format!("--skip-step {}", s)),
        );
    }
    m
}

fn build_expr_ctx(git_status: &GitStatus) -> expr::Context {
    let mut expr_ctx = EXPR_CTX.clone();
    if let Ok(val) = expr::to_value(git_status) {
        expr_ctx.insert("git", val);
    }
    expr_ctx
}

fn can_exit_early(
    groups: &[StepGroup],
    files: &BTreeSet<PathBuf>,
    run_type: RunType,
    skip_steps: &IndexMap<String, SkipReason>,
) -> bool {
    let files = files.iter().cloned().collect::<Vec<_>>();
    groups.iter().all(|g| {
        g.steps.iter().all(|(_, s)| {
            // Reuse job builder to determine if this step has any runnable work
            s.build_step_jobs(&files, run_type, &Default::default(), skip_steps)
                .is_ok_and(|jobs| jobs.iter().all(|j| j.skip_reason.is_some()))
        })
    })
}

fn skip_reason_to_reason(reason: &SkipReason) -> Reason {
    let (kind, detail) = match reason {
        SkipReason::DisabledByEnv(src) => {
            (ReasonKind::EnvExclude, Some(format!("disabled via {src}")))
        }
        SkipReason::DisabledByCli(src) => {
            (ReasonKind::CliExclude, Some(format!("disabled via {src}")))
        }
        SkipReason::DisabledByConfig => (
            ReasonKind::ConfigExclude,
            Some("disabled via skip configuration".to_string()),
        ),
        SkipReason::MissingRequiredEnv(envs) => (
            ReasonKind::MissingRequiredEnv,
            Some(format!("missing: {}", envs.join(", "))),
        ),
        SkipReason::ProfileNotEnabled(profiles) => {
            let detail = if profiles.is_empty() {
                "required profile not enabled".to_string()
            } else {
                format!("required profile(s) not enabled: {}", profiles.join(", "))
            };
            (ReasonKind::ProfileExclude, Some(detail))
        }
        SkipReason::ProfileExplicitlyDisabled => (
            ReasonKind::ProfileExclude,
            Some("disabled by active profile".to_string()),
        ),
        SkipReason::NoCommandForRunType(rt) => {
            let rt_str = match rt {
                RunType::Check => "check",
                RunType::Fix => "fix",
            };
            (
                ReasonKind::NoCommand,
                Some(format!("no command defined for run type: {rt_str}")),
            )
        }
        SkipReason::NoFilesToProcess => (
            ReasonKind::FilterNoMatch,
            Some("no files matched filters".to_string()),
        ),
        SkipReason::ConditionFalse => (
            ReasonKind::ConditionFalse,
            Some("condition evaluated to false".to_string()),
        ),
    };
    Reason {
        kind,
        detail,
        data: HashMap::new(),
    }
}
