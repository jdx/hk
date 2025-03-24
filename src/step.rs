use crate::{Result, error::Error, step_job::StepJob, step_response::StepResponse};
use crate::{glob, settings::Settings};
use crate::{step_context::StepContext, tera};
use ensembler::CmdLineRunner;
use eyre::{WrapErr, eyre};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fmt, sync::Arc};

use serde_with::serde_as;

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[serde_as]
pub struct Step {
    pub r#type: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub profiles: Option<Vec<String>>,
    #[serde(default)]
    pub exclusive: bool,
    pub depends: Vec<String>,
    #[serde(default)]
    pub check_first: bool,
    #[serde(default)]
    pub batch: bool,
    #[serde(default)]
    pub stomp: bool,
    pub run: Option<String>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub glob: Option<Vec<String>>,
    pub check: Option<String>,
    pub check_diff: Option<String>,
    pub check_list_files: Option<String>,
    pub check_extra_args: Option<String>,
    pub fix: Option<String>,
    pub fix_extra_args: Option<String>,
    pub workspace_indicator: Option<String>,
    pub prefix: Option<String>,
    pub dir: Option<String>,
    pub env: IndexMap<String, String>,
    pub root: Option<PathBuf>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub stage: Option<Vec<String>>,
    #[serde(default)]
    pub linter_dependencies: IndexMap<String, Vec<String>>,
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
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckType {
    Check,
    ListFiles,
    Diff,
}

impl Step {
    pub fn fix() -> Self {
        Self {
            r#type: Some("fix".to_string()),
            ..Default::default()
        }
    }
    pub fn check() -> Self {
        Self {
            r#type: Some("check".to_string()),
            ..Default::default()
        }
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
        let pr = self.build_pr();
        let (Some(mut run), extra) = (match job.run_type {
            RunType::Check(CheckType::Check) => {
                (self.check.clone(), self.check_extra_args.as_ref())
            }
            RunType::Check(CheckType::Diff) => {
                (self.check_diff.clone(), self.check_extra_args.as_ref())
            }
            RunType::Check(CheckType::ListFiles) => (
                self.check_list_files.clone(),
                self.check_extra_args.as_ref(),
            ),
            RunType::Fix => (self.fix.clone(), self.fix_extra_args.as_ref()),
            RunType::Run => (self.run.clone(), None),
        }) else {
            warn!("{}: no run command", self);
            return Ok(rsp);
        };
        if let Some(prefix) = &self.prefix {
            run = format!("{} {}", prefix, run);
        }
        if let Some(extra) = extra {
            run = format!("{} {}", run, extra);
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
        pr.set_message(format!(
            "{} – {} – {}",
            file_msg(&job.files),
            self.glob.as_ref().unwrap_or(&vec![]).join(" "),
            run
        ));
        if log::log_enabled!(log::Level::Trace) {
            for file in &job.files {
                trace!("{self}: {}", file.display());
            }
        }
        let mut cmd = CmdLineRunner::new("sh")
            .arg("-c")
            .arg(&run)
            .with_pr(pr.clone());
        if let Some(dir) = &self.dir {
            cmd = cmd.current_dir(dir);
        }
        for (key, value) in &self.env {
            cmd = cmd.env(key, value);
        }
        match cmd.execute().await {
            Ok(_) => {}
            Err(err) => {
                if let ensembler::Error::ScriptFailed(_, _, result) = &err {
                    if let RunType::Check(CheckType::ListFiles) = job.run_type {
                        let stdout = result.stdout.clone();
                        return Err(Error::CheckListFailed {
                            source: eyre!("{}", err),
                            stdout,
                        })?;
                    }
                }
                return Err(err).wrap_err(run);
            }
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
        if rsp.files_to_add.is_empty() {
            pr.finish_with_message("".to_string());
        } else {
            pr.finish_with_message(format!("{} modified", file_msg(&rsp.files_to_add)));
        }
        Ok(rsp)
    }

    fn build_pr(&self) -> Arc<Box<dyn clx::SingleReport>> {
        let mpr = clx::MultiProgressReport::get();
        mpr.add(&self.name)
    }

    pub fn available_run_type(&self, run_type: RunType) -> Option<RunType> {
        match (
            run_type,
            self.check.is_some(),
            self.fix.is_some(),
            self.run.is_some(),
        ) {
            (RunType::Check(_), true, _, _) => Some(RunType::Check(CheckType::Check)),
            (RunType::Fix, _, true, _) => Some(RunType::Fix),
            (_, _, _, true) => Some(RunType::Run),
            (_, false, true, _) => Some(RunType::Fix),
            (_, true, false, _) => Some(RunType::Check(CheckType::Check)),
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

    pub fn is_profile_enabled(&self) -> bool {
        // Check if step should be skipped based on HK_SKIP_STEPS
        if crate::env::HK_SKIP_STEPS.contains(&self.name) {
            debug!("{self}: skipping step due to HK_SKIP_STEPS");
            return false;
        }
        let settings = Settings::get();
        let enabled_profiles = settings.enabled_profiles();
        if let Some(enabled) = self.enabled_profiles() {
            let missing_profiles = enabled.difference(&enabled_profiles).collect::<Vec<_>>();
            if !missing_profiles.is_empty() {
                let missing_profiles = missing_profiles.iter().join(", ");
                debug!("{self}: skipping step due to missing profile: {missing_profiles}");
                return false;
            }
        }
        if let Some(disabled) = self.disabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let disabled_profiles = disabled.intersection(&enabled_profiles).collect::<Vec<_>>();
            if !disabled_profiles.is_empty() {
                let disabled_profiles = disabled_profiles.iter().join(", ");
                debug!("{self}: skipping step due to disabled profile: {disabled_profiles}");
                return false;
            }
        }
        true
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
}
