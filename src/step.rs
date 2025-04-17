use crate::env;
use crate::{Result, error::Error, step_job::StepJob};
use crate::{glob, settings::Settings};
use crate::{step_context::StepContext, tera};
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use ensembler::CmdLineRunner;
use eyre::{WrapErr, eyre};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::sync::{Arc, LazyLock};
use std::{collections::HashSet, path::PathBuf};
use std::{fmt, process::Stdio};
use tokio::sync::OwnedSemaphorePermit;

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
#[serde_as]
pub struct Step {
    #[serde(default)]
    pub name: String,
    pub profiles: Option<Vec<String>>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    #[serde(default)]
    pub glob: Option<Vec<String>>,
    #[serde(default)]
    pub interactive: bool,
    pub depends: Vec<String>,
    pub check: Option<String>,
    pub check_list_files: Option<String>,
    pub check_diff: Option<String>,
    pub fix: Option<String>,
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
    pub exclude: Option<Vec<String>>,
    #[serde(default)]
    pub exclusive: bool,
    pub root: Option<PathBuf>,
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Text,
    Binary,
    Executable,
    NotExecutable,
    Symlink,
    NotSymlink,
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

impl Step {
    pub(crate) fn init(&mut self, name: &str) {
        self.name = name.to_string();
        if self.interactive {
            self.exclusive = true;
        }
    }

    pub fn run_cmd(&self, run_type: RunType) -> Option<&str> {
        match run_type {
            RunType::Check(c) => match c {
                CheckType::Check => self.check.as_deref(),
                CheckType::Diff => self.check_diff.as_deref(),
                CheckType::ListFiles => self.check_list_files.as_deref(),
            }
            .or(self.check.as_deref())
            .or(self.check_list_files.as_deref())
            .or(self.check_diff.as_deref()),
            RunType::Fix => self
                .fix
                .as_deref()
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

    pub fn is_profile_enabled(&self) -> bool {
        is_profile_enabled(
            &self.name,
            self.enabled_profiles(),
            self.disabled_profiles(),
        )
    }

    pub(crate) fn build_step_progress(&self) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body("{{spinner()}} {{name}} {% if message %}– {{message | flex}}{% elif files %}– {{files}}{% endif %}")
            .body_text(Some(
                "{% if message %}{{spinner()}} {{name}} – {{message}}{% elif files %}{{spinner()}} {{name}} – {{files}}{% endif %}",
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
                if let Some(dir) = dir.parent() {
                    dirs.retain(|d| !d.starts_with(dir));
                }
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
            for f in files.iter_mut() {
                // strip the dir prefix from the file path
                *f = f.strip_prefix(dir).unwrap_or(f).to_path_buf();
            }
        }
        if let Some(glob) = &self.glob {
            files = glob::get_matches(glob, &files)?;
        }
        if let Some(exclude) = &self.exclude {
            let excluded = glob::get_matches(exclude, &files)?
                .into_iter()
                .collect::<HashSet<_>>();
            files.retain(|f| !excluded.contains(f));
        }
        Ok(files)
    }

    pub(crate) fn build_step_jobs(
        &self,
        files: &[PathBuf],
        run_type: RunType,
        files_in_contention: &HashSet<PathBuf>,
    ) -> Result<Vec<StepJob>> {
        let files = self.filter_files(files)?;
        if files.is_empty() && (self.glob.is_some() || self.dir.is_some() || self.exclude.is_some())
        {
            debug!("{self}: no file matches for step");
            return Ok(Default::default());
        }
        let mut jobs = if let Some(workspace_indicators) = self.workspaces_for_files(&files)? {
            let job = StepJob::new(Arc::new((*self).clone()), files.clone(), run_type);
            workspace_indicators
                .into_iter()
                .map(|workspace_indicator| {
                    job.clone().with_workspace_indicator(workspace_indicator)
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
            &*ctx.hook_ctx.files_in_contention.lock().await,
        )?;
        if jobs.is_empty() {
            ctx.hook_ctx.dec_total_jobs(1);
            debug!("{self}: no jobs to run");
            return Ok(());
        }
        ctx.status_started();
        ctx.set_jobs_total(jobs.len());
        ctx.hook_ctx.inc_total_jobs(jobs.len() - 1);
        jobs[0].status_start(&ctx, semaphore).await?;
        let mut set = tokio::task::JoinSet::new();
        for job in jobs {
            let ctx = ctx.clone();
            let step = self.clone();
            let mut job = job;
            set.spawn(async move {
                if let Some(condition) = &step.condition {
                    if EXPR_ENV.eval(condition, &EXPR_CTX).unwrap() == expr::Value::Bool(false) {
                        return Ok(());
                    }
                }
                if job.check_first {
                    let prev_run_type = job.run_type;
                    job.run_type = RunType::Check(step.check_type());
                    debug!("{step}: running check step first due to fix step contention");
                    match step.run(&ctx, &mut job).await {
                        Ok(()) => {
                            debug!("{step}: successfully ran check step first");
                            return Ok(());
                        }
                        Err(e) => {
                            if let Some(Error::CheckListFailed { source, stdout }) =
                                e.downcast_ref::<Error>()
                            {
                                debug!("{step}: failed check step first: {source}");
                                let filtered_files: HashSet<PathBuf> =
                                    stdout.lines().map(|p| match PathBuf::from(p).canonicalize() {
                                        Ok(p) => p,
                                        Err(e) => {
                                            warn!("{step}: failed to canonicalize file: {e}");
                                            PathBuf::from(p)
                                        }
                                    }).collect();
                                let files: IndexSet<PathBuf> = job.files.into_iter().filter(|f| {
                                    let f = match f.canonicalize() {
                                        Ok(p) => p,
                                        Err(e) => {
                                            warn!("{step}: failed to canonicalize file: {e}");
                                            f.to_path_buf()
                                        }
                                    };
                                    filtered_files.contains(&f)
                                }).collect();
                                for f in filtered_files.into_iter().filter(|f| !files.contains(f)) {
                                    warn!("{step}: file in check_list_files not found in original files: {}", f.display());
                                }
                                job.files = files.into_iter().collect();
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
                result
            });
        }
        let mut result = Ok(());
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(rsp)) => {
                    result = result.and(Ok(rsp));
                }
                Ok(Err(err)) => {
                    ctx.hook_ctx.failed.cancel();
                    result = result.and(Err(err));
                    // TODO: abort all jobs after a timeout
                    // tokio::spawn(async move {
                    //     tokio::time::sleep(Duration::from_secs(5)).await;
                    //     set.abort_all();
                    // });
                    // if child.is_running() {
                    //     child.set_status(clx::progress::ProgressStatus::DoneCustom(
                    //         style::eyellow("▲").to_string(),
                    //     ));
                    // }
                }
                Err(e) => {
                    std::panic::resume_unwind(e.into_panic());
                }
            }
        }
        if ctx.hook_ctx.failed.is_cancelled() {
            ctx.status_aborted();
        } else if result.is_ok() {
            ctx.status_finished();
        }
        ctx.depends.mark_done(&self.name)?;
        result
    }

    async fn wait_for_depends(
        &self,
        ctx: &StepContext,
        mut semaphore: Option<OwnedSemaphorePermit>,
    ) -> Result<OwnedSemaphorePermit> {
        for dep in &self.depends {
            if !ctx.depends.is_done(dep) {
                warn!("WAITING FOR {}", dep);
                semaphore.take(); // release semaphore for another step
            }
            ctx.depends.wait_for(dep).await?;
        }
        match semaphore {
            Some(semaphore) => Ok(semaphore),
            None => Ok(ctx.hook_ctx.semaphore().await),
        }
    }

    pub(crate) async fn run(&self, ctx: &StepContext, job: &mut StepJob) -> Result<()> {
        if ctx.hook_ctx.failed.is_cancelled() {
            trace!("{self}: skipping step due to previous failure");
            return Ok(());
        }
        job.progress = Some(job.build_progress(ctx));
        if job.status.is_pending() {
            let semaphore = ctx.hook_ctx.semaphore().await;
            job.status_start(ctx, semaphore).await?;
        }
        let mut tctx = job.tctx(&ctx.hook_ctx.tctx);
        tctx.with_globs(self.glob.as_ref().unwrap_or(&vec![]));
        tctx.with_files(&job.files);
        let file_msg = |files: &[PathBuf]| {
            format!(
                "{} file{}",
                files.len(),
                if files.len() == 1 { "" } else { "s" }
            )
        };
        let Some(mut run) = self.run_cmd(job.run_type).map(|s| s.to_string()) else {
            eyre::bail!("{}: no run command", self);
        };
        if let Some(prefix) = &self.prefix {
            run = format!("{} {}", prefix, run);
        }
        let files_to_add = if matches!(job.run_type, RunType::Fix) {
            if let Some(stage) = &self.stage {
                let stage = stage
                    .iter()
                    .map(|s| tera::render(s, &tctx).unwrap())
                    .collect_vec();
                glob::get_matches(&stage, &job.files)?
            } else if self.glob.is_some() {
                job.files.clone()
            } else {
                vec![]
            }
            .into_iter()
            .map(|p| {
                (
                    p.metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    p,
                )
            })
            .collect_vec()
        } else {
            vec![]
        };
        let run = tera::render(&run, &tctx).unwrap();
        job.progress.as_ref().unwrap().prop(
            "message",
            &format!(
                "{} – {} – {}",
                file_msg(&job.files),
                self.glob.as_ref().unwrap_or(&vec![]).join(" "),
                run
            ),
        );
        job.progress.as_ref().unwrap().update();
        if log::log_enabled!(log::Level::Trace) {
            for file in &job.files {
                trace!("{self}: {}", file.display());
            }
        }
        let mut cmd = CmdLineRunner::new("sh")
            .arg("-o")
            .arg("errexit")
            .arg("-c")
            .arg(&run)
            .with_pr(job.progress.as_ref().unwrap().clone())
            .with_cancel_token(ctx.hook_ctx.failed.clone())
            .show_stderr_on_error(false);
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
        match cmd.execute().await {
            Ok(_) => {}
            Err(err) => {
                if self.interactive {
                    clx::progress::resume();
                }
                if let ensembler::Error::ScriptFailed(e) = &err {
                    if let RunType::Check(CheckType::ListFiles) = job.run_type {
                        let result = &e.3;
                        let stdout = result.stdout.clone();
                        return Err(Error::CheckListFailed {
                            source: eyre!("{}", err),
                            stdout,
                        })?;
                    }
                }
                if job.check_first && matches!(job.run_type, RunType::Check(_)) {
                    ctx.progress.set_status(ProgressStatus::Warn);
                } else {
                    ctx.progress.set_status(ProgressStatus::Failed);
                }
                return Err(err).wrap_err(run);
            }
        }
        if self.interactive {
            clx::progress::resume();
        }
        let files_to_add = files_to_add
            .into_iter()
            .filter(|(prev_mod, p)| {
                if !p.exists() {
                    return false;
                }
                let Ok(metadata) = p.metadata().and_then(|m| m.modified()) else {
                    return false;
                };
                metadata > *prev_mod
            })
            .map(|(_, p)| p.to_string_lossy().to_string())
            .collect_vec();

        if !files_to_add.is_empty() {
            ctx.hook_ctx.git.lock().await.add(&files_to_add)?;
        }

        ctx.inc_files_added(files_to_add.len());
        ctx.decrement_job_count();
        ctx.hook_ctx.inc_completed_jobs(1);
        job.status_finished().await?;
        Ok(())
    }
}

fn is_profile_enabled(
    name: &str,
    enabled: Option<IndexSet<String>>,
    disabled: Option<IndexSet<String>>,
) -> bool {
    let settings = Settings::get();
    let enabled_profiles = settings.enabled_profiles();
    if let Some(enabled) = enabled {
        let missing_profiles = enabled.difference(&enabled_profiles).collect::<Vec<_>>();
        if !missing_profiles.is_empty() {
            let missing_profiles = missing_profiles.iter().join(", ");
            debug!("{name}: skipping step due to missing profile: {missing_profiles}");
            return false;
        }
    }
    if let Some(disabled) = disabled {
        let enabled_profiles = settings.enabled_profiles();
        let disabled_profiles = disabled.intersection(&enabled_profiles).collect::<Vec<_>>();
        if !disabled_profiles.is_empty() {
            let disabled_profiles = disabled_profiles.iter().join(", ");
            debug!("{name}: skipping step due to disabled profile: {disabled_profiles}");
            return false;
        }
    }
    true
}

static EXPR_CTX: LazyLock<expr::Context> = LazyLock::new(expr::Context::default);

static EXPR_ENV: LazyLock<expr::Environment> = LazyLock::new(|| {
    let mut env = expr::Environment::default();
    env.add_function("exec", |c| {
        let out = xx::process::sh(c.args[0].as_string().unwrap()).unwrap();
        Ok(expr::Value::String(out))
    });
    env
});
