use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressOutput, ProgressStatus};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    signal,
    sync::{Mutex, OnceCell, OwnedSemaphorePermit, Semaphore},
};
use tokio_util::sync::CancellationToken;

use crate::{
    Result, env,
    file_rw_locks::FileRwLocks,
    git::{Git, GitStatus, StashMethod},
    glob,
    hook_options::HookOptions,
    settings::Settings,
    step::{CheckType, EXPR_CTX, RunType, Step},
    step_context::StepContext,
    step_group::{StepGroup, StepGroupContext},
    timings::TimingRecorder,
    ui::style,
    version,
};

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
    pub timing: Option<Arc<TimingRecorder>>,
}

impl HookContext {
    pub fn new(
        files: impl IntoIterator<Item = PathBuf>,
        git: Arc<Mutex<Git>>,
        groups: Vec<StepGroup>,
        tctx: crate::tera::Context,
        run_type: RunType,
        hk_progress: Option<Arc<ProgressJob>>,
    ) -> Self {
        let settings = Settings::get();
        let expr_ctx = EXPR_CTX.clone();
        let timing = env::HK_TIMING_JSON
            .as_ref()
            .map(|path| Arc::new(TimingRecorder::new(path.clone())));
        Self {
            file_locks: FileRwLocks::new(files),
            git,
            hk_progress,
            total_jobs: std::sync::Mutex::new(groups.iter().map(|g| g.steps.len()).sum()),
            completed_jobs: std::sync::Mutex::new(0),
            groups,
            tctx,
            run_type,
            step_contexts: std::sync::Mutex::new(Default::default()),
            files_in_contention: std::sync::Mutex::new(Default::default()),
            semaphore: Arc::new(Semaphore::new(settings.jobs.get())),
            failed: CancellationToken::new(),
            expr_ctx: std::sync::Mutex::new(expr_ctx),
            timing,
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

    fn get_step_groups(&self, run_type: RunType, opts: &HookOptions) -> Vec<StepGroup> {
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
        let step_ok = |step: &Step| {
            if step.run_cmd(run_type).is_none() {
                debug!("{step}: skipping step due to no available run type");
                false
            } else if env::HK_SKIP_STEPS.contains(&step.name) {
                debug!("{step}: skipping step due to HK_SKIP_STEPS");
                false
            } else {
                step.is_profile_enabled()
            }
        };
        let steps = steps
            .into_iter()
            .filter_map(|s| match s {
                StepOrGroup::Step(ref step) => step_ok(step).then_some(s),
                StepOrGroup::Group(mut group) => {
                    group.steps.retain(|_, s| step_ok(s));
                    Some(StepOrGroup::Group(group))
                }
            })
            .collect_vec();
        StepGroup::build_all(steps)
    }

    pub async fn plan(&self, opts: HookOptions) -> Result<()> {
        let run_type = self.run_type(&opts);
        let groups = self.get_step_groups(run_type, &opts);
        let repo = Arc::new(Mutex::new(Git::new()?));
        let git_status = OnceCell::new();
        let stash_method = env::HK_STASH.or(self.stash).unwrap_or(StashMethod::None);
        let progress = ProgressJobBuilder::new()
            .status(ProgressStatus::Hide)
            .build();
        let files = self
            .file_list(&opts, repo.clone(), &git_status, stash_method, &progress)
            .await?;
        if files.is_empty() && can_exit_early(&groups, &files, run_type) {
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

    pub async fn run(&self, opts: HookOptions) -> Result<()> {
        let settings = Settings::get();
        if env::HK_SKIP_HOOK.contains(&self.name) {
            warn!("{}: skipping hook due to HK_SKIP_HOOK", &self.name);
            return Ok(());
        }
        let run_type = self.run_type(&opts);
        let repo = Arc::new(Mutex::new(Git::new()?));
        let git_status = OnceCell::new();
        let groups = self.get_step_groups(run_type, &opts);
        let stash_method = env::HK_STASH.or(self.stash).unwrap_or(StashMethod::None);
        let hk_progress = self.start_hk_progress(run_type, groups.len());
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

        if files.is_empty() && can_exit_early(&groups, &files, run_type) {
            info!("no files to run");
            if let Some(hk_progress) = &hk_progress {
                hk_progress.set_status(ProgressStatus::Hide);
            }
            return Ok(());
        }
        let hook_ctx = Arc::new(HookContext::new(
            files,
            repo.clone(),
            groups,
            opts.tctx,
            run_type,
            hk_progress,
        ));

        watch_for_ctrl_c(hook_ctx.failed.clone());

        if stash_method != StashMethod::None {
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status(None))
                .await?;
            repo.lock()
                .await
                .stash_unstaged(&file_progress, stash_method, git_status)?;
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
        if let Some(timing) = &hook_ctx.timing {
            if let Err(err) = timing.write_json() {
                warn!("Failed to write timing JSON: {err}");
            }
        }
        result
    }

    async fn file_list(
        &self,
        opts: &HookOptions,
        repo: Arc<Mutex<Git>>,
        git_status: &OnceCell<GitStatus>,
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
                let git_status = git_status
                    .get_or_try_init(async || repo.lock().await.status(None))
                    .await?;
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
                let git_status = git_status
                    .get_or_try_init(async || repo.lock().await.status(None))
                    .await?;
                all_files.extend(git_status.untracked_files.iter().cloned());
            }
            all_files
        } else if stash {
            file_progress.prop("message", "Fetching staged files");
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status(None))
                .await?;
            git_status.staged_files.iter().cloned().collect()
        } else {
            file_progress.prop("message", "Fetching modified files");
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status(None))
                .await?;
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
            .body("{{hk}}{{hook}}{{message}}  {{progress_bar(width=40)}}")
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

// TimingRecorder moved to timings.rs

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

fn can_exit_early(groups: &[StepGroup], files: &BTreeSet<PathBuf>, run_type: RunType) -> bool {
    let files = files.iter().cloned().collect::<Vec<_>>();
    groups.iter().all(|g| {
        g.steps.iter().all(|(_, s)| {
            s.build_step_jobs(&files, run_type, &Default::default())
                .is_ok_and(|jobs| jobs.is_empty())
        })
    })
}
