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
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
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
    pub run: Option<String>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub glob: Option<Vec<String>>,
    pub check: Option<String>,
    pub check_extra_args: Option<String>,
    pub fix: Option<String>,
    pub fix_extra_args: Option<String>,
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
    Run,
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
    pub async fn run(&self, mut ctx: StepContext) -> Result<StepContext> {
        let mut tctx = tera::Context::default();
        tctx.with_globs(self.glob.as_ref().unwrap_or(&vec![]));
        tctx.with_files(&ctx.files);
        let pr = self.build_pr();
        let (Some(mut run), extra) = (match ctx.run_type {
            RunType::Check => (self.check.clone(), self.check_extra_args.as_ref()),
            RunType::Fix => (self.fix.clone(), self.fix_extra_args.as_ref()),
            RunType::Run => (self.run.clone(), None),
        }) else {
            warn!("{}: no run command", self);
            return Ok(ctx);
        };
        if let Some(extra) = extra {
            run = format!("{} {}", run, extra);
        }
        let files_to_add = if matches!(ctx.run_type, RunType::Fix) {
            if let Some(stage) = &self.stage {
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
        pr.set_message(run.clone());
        CmdLineRunner::new("sh")
            .arg("-c")
            .arg(run)
            .with_pr(pr.clone())
            .execute()
            .await
            .into_diagnostic()?;
        ctx.files_to_add = files_to_add
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
        match (run_type, self.check.is_some(), self.fix.is_some(), self.run.is_some()) {
            (RunType::Check, true, _, _) => Some(RunType::Check),
            (RunType::Fix, _, true, _) => Some(RunType::Fix),
            (_, _, _, true) => Some(RunType::Run),
            (_, false, true, _) => Some(RunType::Fix),
            (_, true, false, _) => Some(RunType::Check),
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
}

#[derive(Clone)]
pub struct StepContext {
    pub run_type: RunType,
    pub files: Vec<PathBuf>,
    pub file_locks: IndexMap<PathBuf, Arc<RwLock<()>>>,
    pub files_to_add: Vec<PathBuf>,
}
