use crate::env;
use crate::ui::style;
use crate::{Result, error::Error, step_job::StepJob, step_response::StepResponse};
use crate::{glob, settings::Settings};
use crate::{step_context::StepContext, tera};
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use ensembler::CmdLineRunner;
use eyre::{WrapErr, eyre};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{collections::{HashSet, VecDeque}, path::PathBuf};
use std::sync::Arc;
use std::{fmt, process::Stdio};

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
#[serde_as]
pub struct LinterStep {
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

impl fmt::Display for LinterStep {
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

impl LinterStep {
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
            .or(self.fix.as_deref()),
            RunType::Fix => self
                .fix
                .as_deref()
                .or_else(|| self.run_cmd(RunType::Check(CheckType::Check))),
        }
    }

    pub fn available_run_type(&self, run_type: RunType) -> Option<RunType> {
        match (run_type, self.check.is_some(), self.fix.is_some()) {
            (RunType::Check(_), true, _) => Some(RunType::Check(CheckType::Check)),
            (RunType::Fix, _, _) => Some(RunType::Fix),
            (_, false, true) => Some(RunType::Fix),
            _ => None,
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

    pub fn is_enabled(&self, run_type: RunType) -> bool {
        if env::HK_SKIP_STEPS.contains(&self.name) {
            debug!("{self}: skipping step due to HK_SKIP_STEPS");
            return false;
        }
        if !self.is_profile_enabled() {
            debug!("{self}: skipping step due to profile not being enabled");
            return false;
        }
        if self.available_run_type(run_type).is_none() {
            debug!("{self}: skipping step due to no available run type");
            return false;
        }
        true
    }

    pub fn is_profile_enabled(&self) -> bool {
        is_profile_enabled(
            &self.name,
            self.enabled_profiles(),
            self.disabled_profiles(),
        )
    }

    pub fn build_jobs(self: &Arc<Self>, mut files: Vec<PathBuf>, run_type: RunType) -> Result<VecDeque<StepJob>> {
        let Some(run_type) = self.available_run_type(run_type) else {
            debug!("{self}: skipping step due to no available run type");
            return Ok(Default::default());
        };
        if !self.is_enabled(run_type) {
            debug!("{self}: skipping step due to not being enabled");
            return Ok(Default::default());
        }
        let settings = Settings::get();
        if let Some(dir) = &self.dir {
            files.retain(|f| f.starts_with(dir));
            if files.is_empty() {
                debug!("{self}: no matches for step in {dir}");
                return Ok(Default::default())
            }
            for f in files.iter_mut() {
                // strip the dir prefix from the file path
                *f = f.strip_prefix(dir).unwrap_or(f).to_path_buf();
            }
        }
        if let Some(glob) = &self.glob {
            files = glob::get_matches(glob, &files)?;
            if files.is_empty() {
                debug!("{self}: no matches for step");
                return Ok(Default::default());
            }
        }
        if let Some(exclude) = &self.exclude {
            let excluded = glob::get_matches(exclude, &files)?
                .into_iter()
                .collect::<HashSet<_>>();
            files.retain(|f| !excluded.contains(f));
        }
        let jobs = if let Some(workspace_indicators) = self.workspaces_for_files(&files)? {
            let job = StepJob::new(self.clone(), files.clone(), run_type);
            workspace_indicators
                .into_iter()
                .map(|workspace_indicator| {
                    job.clone().with_workspace_indicator(workspace_indicator)
                })
                .collect()
        } else if self.batch {
            files
                .chunks((files.len() / settings.jobs().get()).max(1))
                .map(|chunk| StepJob::new(self.clone(), chunk.to_vec(), run_type))
                .collect()
        } else {
            VecDeque::from([StepJob::new(self.clone(), files.clone(), run_type)])
        };
        Ok(jobs)
    }

    pub(crate) fn build_step_progress(&self) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body(vec![
                "{{spinner()}} {{name}} {% if message %}– {{message | flex}}{% endif %}"
                    .to_string(),
            ])
            .body_text(Some(vec![
                "{% if message %}{{spinner()}} {{name}} – {{message}}{% endif %}".to_string(),
            ]))
            .prop("name", &self.name)
            .status(ProgressStatus::RunningCustom(style::edim("❯").to_string()))
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

    pub(crate) fn can_run(&self, ctx: &StepContext) -> bool {
        for dep in &self.depends {
            if !ctx.depends.is_done(dep) {
                return false;
            }
        }
        true
    }

    pub(crate) async fn run(&self, ctx: &StepContext, job: &StepJob) -> Result<StepResponse> {
        let mut rsp = StepResponse::default();
        let mut tctx = job.tctx(&ctx.tctx);
        tctx.with_globs(self.glob.as_ref().unwrap_or(&vec![]));
        tctx.with_files(&job.files);
        let file_msg = |files: &[PathBuf]| {
            format!(
                "{} file{}",
                files.len(),
                if files.len() == 1 { "" } else { "s" }
            )
        };
        let pr = job.build_progress(ctx);
        let Some(mut run) = self.run_cmd(job.run_type).map(|s| s.to_string()) else {
            warn!("{}: no run command", self);
            return Ok(rsp);
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
        pr.prop(
            "message",
            &format!(
                "{} – {} – {}",
                file_msg(&job.files),
                self.glob.as_ref().unwrap_or(&vec![]).join(" "),
                run
            ),
        );
        pr.update();
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
            .with_pr(pr.clone())
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
        rsp.files_to_add = files_to_add
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
            .map(|(_, p)| p)
            .collect_vec();
        ctx.inc_files_added(rsp.files_to_add.len());
        ctx.decrement_job_count();
        ctx.update_progress();
        pr.set_status(ProgressStatus::Done);
        Ok(rsp)
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
