use crate::timings::StepTimingGuard;
use crate::{Result, error::Error, step_job::StepJob};
use crate::{env, step_job::StepJobStatus};
use crate::{glob, settings::Settings};
use crate::{step_context::StepContext, tera, ui::style};
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use ensembler::CmdLineRunner;
use eyre::{WrapErr, eyre};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, OneOrMany, PickFirst, serde_as};
use shell_quote::QuoteInto;
use shell_quote::QuoteRefExt;
use std::{collections::HashSet, fmt::Display, path::PathBuf, str::FromStr};
use std::{
    ffi::OsString,
    sync::{Arc, LazyLock},
};
use std::{fmt, process::Stdio};
use tokio::sync::OwnedSemaphorePermit;
use xx::file::display_path;

#[serde_as]
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
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
    pub exclude: Option<Vec<String>>,
    #[serde(default)]
    pub exclusive: bool,
    pub root: Option<PathBuf>,
    #[serde(default)]
    pub hide: bool,
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
        if let Some(glob) = &self.glob {
            // when dir is set, globs are relative to that dir only
            if let Some(dir) = &self.dir {
                let dir_globs = glob
                    .iter()
                    .map(|g| format!("{}/{}", dir.trim_end_matches('/'), g))
                    .collect::<Vec<_>>();
                files = glob::get_matches(&dir_globs, &files)?;
            } else {
                files = glob::get_matches(glob, &files)?;
            }
        }
        if let Some(exclude) = &self.exclude {
            // when dir is set, excludes are relative to that dir only
            let excluded = if let Some(dir) = &self.dir {
                let dir_excludes = exclude
                    .iter()
                    .map(|g| format!("{}/{}", dir.trim_end_matches('/'), g))
                    .collect::<Vec<_>>();
                glob::get_matches(&dir_excludes, &files)?
                    .into_iter()
                    .collect::<HashSet<_>>()
            } else {
                glob::get_matches(exclude, &files)?
                    .into_iter()
                    .collect::<HashSet<_>>()
            };
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
                .chunks((files.len() / Settings::get().jobs.get()).max(1))
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
        // Handle skip conditions at the step level while still surfacing progress to the user
        if env::HK_SKIP_STEPS.contains(&self.name) {
            ctx.progress
                .prop("message", "skipped: disabled via HK_SKIP_STEPS");
            ctx.progress
                .set_status(ProgressStatus::DoneCustom(style::edim("⏭").to_string()));
            ctx.depends.mark_done(&self.name)?;
            return Ok(());
        }
        if !self.is_profile_enabled() {
            ctx.progress.prop("message", "skipped: disabled by profile");
            ctx.progress
                .set_status(ProgressStatus::DoneCustom(style::edim("⏭").to_string()));
            ctx.depends.mark_done(&self.name)?;
            return Ok(());
        }
        if self.run_cmd(ctx.hook_ctx.run_type).is_none() {
            ctx.progress
                .prop("message", "skipped: no command for run type");
            ctx.progress
                .set_status(ProgressStatus::DoneCustom(style::edim("⏭").to_string()));
            ctx.depends.mark_done(&self.name)?;
            debug!("{self}: no available run type");
            return Ok(());
        }
        let files = ctx.hook_ctx.files();
        let ctx = Arc::new(ctx);
        let mut jobs = self.build_step_jobs(
            &files,
            ctx.hook_ctx.run_type,
            &ctx.hook_ctx.files_in_contention.lock().unwrap(),
        )?;
        if let Some(job) = jobs.first_mut() {
            job.semaphore = Some(semaphore);
        } else {
            // // No jobs due to no matching files or other filters; surface a skip message
            // ctx.progress
            //     .prop("message", &format!("skipped: no files to process"));
            // ctx.progress
            //     .set_status(ProgressStatus::DoneCustom(style::edim("⏭").to_string()));
            ctx.depends.mark_done(&self.name)?;
            debug!("{self}: no files to run");
            return Ok(());
        }
        ctx.set_jobs_total(jobs.len());
        let mut set = tokio::task::JoinSet::new();
        for job in jobs {
            let ctx = ctx.clone();
            let step = self.clone();
            let mut job = job;
            set.spawn(async move {
                ctx.hook_ctx.inc_total_jobs(1);
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
                                    stdout.lines().map(|p| try_canonicalize(&PathBuf::from(p))).collect();
                                let files: IndexSet<PathBuf> = job.files.into_iter().filter(|f| {
                                    filtered_files.contains(&try_canonicalize(f))
                                }).collect();
                                let canonicalized_files: IndexSet<PathBuf> = files.iter().map(try_canonicalize).collect();
                                for f in filtered_files.into_iter().filter(|f| !canonicalized_files.contains(f)) {
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
        while let Some(res) = set.join_next().await {
            ctx.hook_ctx.inc_completed_jobs(1);
            match res {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    ctx.status_errored(&format!("{err}"));
                    return Err(err);
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
        if matches!(ctx.hook_ctx.run_type, RunType::Fix) {
            // Build stage pathspecs; if `dir` is set, stage entries are relative to it
            let stage: Vec<OsString> = self
                .stage
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .map(|s| tera::render(s, &ctx.hook_ctx.tctx).unwrap())
                .map(|s| {
                    if let Some(dir) = &self.dir {
                        format!("{}/{}", dir.trim_end_matches('/'), s)
                    } else {
                        s
                    }
                })
                .map(OsString::from)
                .collect_vec();
            if !stage.is_empty() {
                let status = ctx.hook_ctx.git.lock().await.status(Some(&stage))?;
                let files = status.unstaged_files.into_iter().collect_vec();
                if !files.is_empty() {
                    ctx.hook_ctx.git.lock().await.add(&files)?;
                    ctx.add_files(&files);
                }
            }
        }
        ctx.status_finished();
        ctx.depends.mark_done(&self.name)?;
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

    pub(crate) async fn run(&self, ctx: &StepContext, job: &mut StepJob) -> Result<()> {
        if ctx.hook_ctx.failed.is_cancelled() {
            trace!("{self}: skipping step due to previous failure");
            return Ok(());
        }
        if let Some(condition) = &self.condition {
            let val = EXPR_ENV.eval(condition, &ctx.hook_ctx.expr_ctx())?;
            debug!("{self}: condition: {condition} = {val}");
            if val == expr::Value::Bool(false) {
                ctx.progress.prop("message", "skipped: condition is false");
                ctx.progress
                    .set_status(ProgressStatus::DoneCustom(style::eblue("⏭").to_string()));
                return Ok(());
            }
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
        tctx.with_globs(self.glob.as_ref().unwrap_or(&vec![]));
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
        let timing_guard = StepTimingGuard::new(ctx.hook_ctx.timing.clone(), self);
        let exec_result = cmd.execute().await;
        timing_guard.finish();
        if self.interactive {
            clx::progress::resume();
        }
        match exec_result {
            Ok(_) => {}
            Err(err) => {
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
        ctx.decrement_job_count();
        job.status_finished()?;
        Ok(())
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
fn is_profile_enabled(
    name: &str,
    enabled: Option<IndexSet<String>>,
    disabled: Option<IndexSet<String>>,
) -> bool {
    let settings = Settings::get();
    if let Some(enabled) = enabled {
        let missing_profiles = enabled
            .difference(&settings.enabled_profiles)
            .collect::<Vec<_>>();
        if !missing_profiles.is_empty() {
            let missing_profiles = missing_profiles.iter().join(", ");
            debug!("{name}: skipping step due to missing profile: {missing_profiles}");
            return false;
        }
        let disabled_profiles = settings
            .disabled_profiles
            .intersection(&enabled)
            .collect_vec();
        if !disabled_profiles.is_empty() {
            let disabled_profiles = disabled_profiles.iter().join(", ");
            debug!("{name}: skipping step due to disabled profile: {disabled_profiles}");
            return false;
        }
    }
    if let Some(disabled) = disabled {
        let disabled_profiles = disabled
            .intersection(&settings.enabled_profiles)
            .collect::<Vec<_>>();
        if !disabled_profiles.is_empty() {
            let disabled_profiles = disabled_profiles.iter().join(", ");
            debug!("{name}: skipping step due to disabled profile: {disabled_profiles}");
            return false;
        }
    }
    true
}

pub static EXPR_CTX: LazyLock<expr::Context> = LazyLock::new(expr::Context::default);

pub static EXPR_ENV: LazyLock<expr::Environment> = LazyLock::new(|| {
    let mut env = expr::Environment::default();

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
