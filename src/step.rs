use crate::glob;
use crate::tera;
use crate::Result;
use ensembler::CmdLineRunner;
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fmt, sync::Arc};
use tokio::sync::RwLock;

use serde_with::{serde_as, OneOrMany};

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub r#type: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub profiles: Option<Vec<String>>,
    #[serde(default)]
    pub exclusive: bool,
    #[serde_as(as = "OneOrMany<_>")]
    pub depends: Vec<String>,
    #[serde(default)]
    pub check_first: bool,
    #[serde(default)]
    pub stomp: bool,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub glob: Option<Vec<String>>,
    pub check: Option<String>,
    pub fix: Option<String>,
    pub check_all: Option<String>,
    pub fix_all: Option<String>,
    pub root: Option<PathBuf>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub stage: Option<Vec<String>>,
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
    Check,
    Fix,
    CheckAll,
    FixAll,
}

impl Step {
    pub async fn run(&self, mut ctx: StepContext) -> Result<StepContext> {
        let mut tctx = tera::Context::default();
        tctx.with_globs(self.glob.as_ref().unwrap_or(&vec![]));
        tctx.with_files(&ctx.files);
        let pr = self.build_pr();
        let Some(run) = (match ctx.run_type {
            RunType::Check => self.check.as_ref(),
            RunType::Fix => self.fix.as_ref(),
            RunType::CheckAll => self.check_all.as_ref(),
            RunType::FixAll => self.fix_all.as_ref(),
        }) else {
            warn!("{}: no run command", self);
            return Ok(ctx);
        };
        let run = tera::render(run, &tctx).unwrap();
        pr.set_message(run.clone());
        CmdLineRunner::new("sh")
            .arg("-c")
            .arg(run)
            .with_pr(pr.clone())
            .execute()
            .await
            .into_diagnostic()?;
        if matches!(ctx.run_type, RunType::Fix | RunType::FixAll) {
            ctx.files_to_add = if let Some(stage) = &self.stage {
                let stage = stage
                    .iter()
                    .map(|s| tera::render(s, &tctx).unwrap())
                    .collect_vec();
                glob::get_matches(&stage, &ctx.files)?
            } else if self.glob.is_some() {
                ctx.files.clone()
            } else {
                vec![]
            }
        }
        if ctx.files_to_add.is_empty() {
            pr.finish_with_message("done".to_string());
        } else {
            pr.finish_with_message(format!(
                "{} file{}",
                ctx.files_to_add.len(),
                if ctx.files_to_add.len() == 1 { "" } else { "s" }
            ));
        }
        Ok(ctx)
    }

    fn build_pr(&self) -> Arc<Box<dyn clx::SingleReport>> {
        let mpr = clx::MultiProgressReport::get();
        mpr.add(&self.name)
    }

    pub fn available_run_type(&self, run_type: RunType) -> Option<RunType> {
        match run_type {
            RunType::CheckAll => match self.check_all.is_some() {
                true => Some(RunType::CheckAll),
                false => self.available_run_type(RunType::Check),
            },
            RunType::FixAll => match self.fix_all.is_some() {
                true => Some(RunType::FixAll),
                false => self
                    .available_run_type(RunType::Fix)
                    .or(self.available_run_type(RunType::CheckAll)),
            },
            RunType::Check => match (self.check.is_some(), self.check_all.is_some()) {
                (true, _) => Some(RunType::Check),
                (_, true) => Some(RunType::CheckAll),
                _ => None,
            },
            RunType::Fix => match (self.fix.is_some(), self.fix_all.is_some()) {
                (true, _) => Some(RunType::Fix),
                (_, true) => Some(RunType::FixAll),
                _ => self.available_run_type(RunType::Check),
            },
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
}

#[derive(Clone)]
pub struct StepContext {
    pub run_type: RunType,
    pub files: Vec<PathBuf>,
    pub file_locks: IndexMap<PathBuf, Arc<RwLock<()>>>,
    pub files_to_add: Vec<PathBuf>,
}
