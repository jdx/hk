use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressOutput, ProgressStatus};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
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
    settings::Settings,
    shell::Shell,
    step::{CheckType, EXPR_CTX, OutputSummary, RunType, Script, Step},
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
    ProfileNotEnabled(Vec<String>),
    ProfileExplicitlyDisabled,
    #[strum(serialize = "no-command-for-run-type")]
    NoCommandForRunType(RunType),
    NoFilesToProcess,
    ConditionFalse,
}

impl SkipReason {
    pub fn message(&self) -> String {
        match self {
            SkipReason::DisabledByEnv(src) | SkipReason::DisabledByCli(src) => {
                format!("skipped: disabled via {src}")
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
    pub stash: Option<StashMethod>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub report: Option<Script>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(tag = "_type", rename_all = "snake_case")]
pub enum StepOrGroup {
    Step(Box<Step>),
    Group(Box<StepGroup>),
}

impl StepOrGroup {
    pub fn init(&mut self, name: &str) {
        match self {
            StepOrGroup::Step(step) => step.init(name),
            StepOrGroup::Group(group) => group.init(name),
        }
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
            semaphore: Arc::new(Semaphore::new(settings.jobs.get())),
            failed: CancellationToken::new(),
            expr_ctx: StdMutex::new(expr_ctx),
            timing: Arc::new(timing),
            skip_steps,
            skipped_steps: StdMutex::new(IndexMap::new()),
            output_by_step: StdMutex::new(IndexMap::new()),
            fix_suggestions: StdMutex::new(Vec::new()),
        }
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.file_locks.files()
    }

    pub fn add_files(&self, files: &[PathBuf]) {
        self.file_locks.add_files(files);
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
    pub fn init(&mut self, hook_name: &str) {
        self.name = hook_name.to_string();
        for (name, step) in self.steps.iter_mut() {
            step.init(name);
        }
    }

    fn run_type(&self, opts: &HookOptions) -> RunType {
        let fix = self.fix.unwrap_or(self.name == "fix");
        if (*env::HK_FIX && fix) || opts.fix {
            RunType::Fix
        } else {
            RunType::Check(CheckType::Check)
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

    pub async fn plan(&self, opts: HookOptions) -> Result<()> {
        let run_type = self.run_type(&opts);
        let groups = self.get_step_groups(&opts);
        let repo = Arc::new(Mutex::new(Git::new()?));
        let git_status = repo.lock().await.status(None)?;
        let stash_method = env::HK_STASH.or(self.stash).unwrap_or(StashMethod::None);
        let progress = ProgressJobBuilder::new()
            .status(ProgressStatus::Hide)
            .build();
        let files = self
            .file_list(&opts, repo.clone(), &git_status, stash_method, &progress)
            .await?;
        if files.is_empty() && can_exit_early(&groups, &files, run_type, &IndexMap::new()) {
            info!("no files to run");
            return Ok(());
        }
        if stash_method != StashMethod::None {
            info!("stashing unstaged changes");
        }
        for group in groups {
            group.plan().await?;
        }
        Ok(())
    }

    #[tracing::instrument(level = "info", name = "hook.run", skip(self, opts), fields(hook = %self.name))]
    pub async fn run(&self, opts: HookOptions) -> Result<()> {
        let settings = Settings::get();
        if env::HK_SKIP_HOOK.contains(&self.name) {
            warn!("{}: skipping hook due to HK_SKIP_HOOK", &self.name);
            return Ok(());
        }
        let run_type = self.run_type(&opts);
        let repo = Arc::new(Mutex::new(Git::new()?));
        let git_status = repo.lock().await.status(None)?;
        let groups = self.get_step_groups(&opts);
        let stash_method = env::HK_STASH.or(self.stash).unwrap_or(StashMethod::None);
        let total_steps: usize = groups.iter().map(|g| g.steps.len()).sum();
        let hk_progress = self.start_hk_progress(run_type, total_steps);
        let file_progress = ProgressJobBuilder::new().body(
            "{{spinner()}} files - {{message}}{% if files is defined %} ({{files}} file{{files|pluralize}}){% endif %}"
        )
        .prop("message", "Fetching git status")
        .start();
        let files = self
            .file_list(
                &opts,
                repo.clone(),
                &git_status,
                stash_method,
                &file_progress,
            )
            .await?;

        let skip_steps = {
            let mut m: IndexMap<String, SkipReason> = IndexMap::new();
            for s in env::HK_SKIP_STEPS.iter() {
                m.insert(
                    s.clone(),
                    SkipReason::DisabledByEnv("HK_SKIP_STEPS".to_string()),
                );
            }
            for s in opts.skip_step.iter() {
                m.insert(
                    s.clone(),
                    SkipReason::DisabledByCli(format!("--skip-step {}", s)),
                );
            }
            m
        };
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
        // Build expression context with the same data under "git"
        let mut expr_ctx = EXPR_CTX.clone();
        if let Ok(val) = expr::to_value(&git_status_for_ctx) {
            expr_ctx.insert("git", val);
        }
        let hook_ctx = Arc::new(HookContext::new(
            files,
            repo.clone(),
            groups,
            tctx,
            expr_ctx,
            run_type,
            hk_progress,
            skip_steps,
        ));

        watch_for_ctrl_c(hook_ctx.failed.clone());

        if stash_method != StashMethod::None {
            repo.lock()
                .await
                .stash_unstaged(&file_progress, stash_method, &git_status)?;
        }

        if hook_ctx.groups.is_empty() {
            info!("no steps to run");
            return Ok(());
        }
        let mut result = Ok(());
        let multiple_groups = hook_ctx.groups.len() > 1;
        for (i, group) in hook_ctx.groups.iter().enumerate() {
            debug!("running group: {i}");
            let mut ctx = StepGroupContext::new(hook_ctx.clone());
            if multiple_groups {
                if let Some(name) = &group.name {
                    ctx = ctx.with_progress(group.build_group_progress(name));
                }
            }
            result = result.and(group.run(ctx).await);
            if settings.fail_fast && result.is_err() {
                break;
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
        // Only show summary if user has enabled the generic warning tag
        if Settings::get().warnings.contains("missing-profiles") {
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
                warn!("  or set {hide_warning_pkl} in .hkrc.pkl");
            }
        }

        // Run hook-level report if configured
        if let Some(report) = &self.report {
            if let Ok(json) = hook_ctx.timing.to_json_string() {
                let mut cmd = Shell::detect().runner();
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
        }
        // Emit collected fix suggestions at the end (after progress bars and summaries)
        let suggestions = hook_ctx.take_fix_suggestions();
        if !suggestions.is_empty() {
            for s in suggestions {
                error!("{}", s);
            }
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
                .cloned()
                .collect()
        };
        for exclude in opts.exclude.as_ref().unwrap_or(&vec![]) {
            let exclude = Path::new(&exclude);
            files.retain(|f| !f.starts_with(exclude));
        }
        if let Some(exclude_glob) = &opts.exclude_glob {
            let f = files.iter().collect::<Vec<_>>();
            let exclude_files = glob::get_matches(exclude_glob, &f)?
                .into_iter()
                .collect::<HashSet<_>>();
            files.retain(|f| !exclude_files.contains(f));
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
    for entry in xx::file::ls(dir)? {
        if entry.is_dir() {
            files.extend(all_files_in_dir(&entry)?);
        } else {
            files.push(entry);
        }
    }
    Ok(files)
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
