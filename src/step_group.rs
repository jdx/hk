use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use eyre::Context;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};

use crate::{
    Result, glob,
    hook::{HookContext, StepOrGroup},
    step::{Pattern, RunType, Script, Step},
    step_context::StepContext,
    step_depends::StepDepends,
};

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[serde_as]
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct StepGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,
    pub name: Option<String>,
    pub workspace_indicator: Option<String>,
    pub prefix: Option<String>,
    pub dir: Option<String>,
    #[serde_as(as = "Option<PickFirst<(_, DisplayFromStr)>>")]
    pub shell: Option<Script>,
    pub stage: Option<Vec<String>>,
    pub exclude: Option<Pattern>,
    #[serde(default)]
    pub steps: IndexMap<String, Step>,
}

pub struct StepGroupContext {
    pub hook_ctx: Arc<HookContext>,
    pub progress: Option<Arc<ProgressJob>>,
    pub fail_fast: bool,
}

impl StepGroupContext {
    pub fn new(hook_ctx: Arc<HookContext>, fail_fast: bool) -> Self {
        Self {
            hook_ctx,
            progress: None,
            fail_fast,
        }
    }
    pub fn with_progress(mut self, progress: Arc<ProgressJob>) -> Self {
        self.progress = Some(progress);
        self
    }
}

impl StepGroup {
    pub fn init(&mut self, name: &str) -> Result<()> {
        self.name = Some(name.to_string());
        let workspace_indicator = self.workspace_indicator.clone();
        let prefix = self.prefix.clone();
        let dir = self.dir.clone();
        let shell = self.shell.clone();
        let stage = self.stage.clone();
        let exclude = self.exclude.clone();

        for (step_name, step) in self.steps.iter_mut() {
            if step.workspace_indicator.is_none() {
                step.workspace_indicator = workspace_indicator.clone();
            }
            if step.prefix.is_none() {
                step.prefix = prefix.clone();
            }
            if step.dir.is_none() {
                step.dir = dir.clone();
            }
            if step.shell.is_none() {
                step.shell = shell.clone();
            }
            if step.stage.is_none() {
                step.stage = stage.clone();
            }
            if step.exclude.as_ref().is_none_or(Pattern::is_empty) {
                step.exclude = exclude.clone();
            }
            step.init(step_name)?;
        }
        Ok(())
    }

    pub fn build_all(steps: Vec<StepOrGroup>) -> Vec<Self> {
        steps
            .into_iter()
            .fold(vec![], |mut groups, step| {
                match step {
                    StepOrGroup::Group(group) => {
                        groups.push(group.steps);
                    }
                    StepOrGroup::Step(step) => {
                        if step.exclusive || groups.is_empty() {
                            groups.push(IndexMap::new());
                        }
                        let exclusive = step.exclusive;
                        groups.last_mut().unwrap().insert(step.name.clone(), *step);
                        if exclusive {
                            groups.push(IndexMap::new());
                        }
                    }
                }
                groups
            })
            .into_iter()
            .filter(|steps| !steps.is_empty())
            .map(|steps| Self {
                _type: None,
                name: None,
                steps,
                ..Default::default()
            })
            .collect()
    }

    pub fn build_group_progress(&self, name: &str) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body("group: {{group}}")
            .prop("group", &name)
            .start()
    }

    pub async fn run(&self, ctx: StepGroupContext) -> Result<()> {
        // timing metadata already pre-populated in HookContext::new
        let depends = Arc::new(StepDepends::new(
            &self
                .steps
                .values()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
        ));
        let mut set = tokio::task::JoinSet::new();
        *ctx.hook_ctx.step_contexts.lock().unwrap() = self
            .steps
            .values()
            .map(|s| {
                (
                    s.name.clone(),
                    Arc::new(StepContext {
                        step: s.clone(),
                        hook_ctx: ctx.hook_ctx.clone(),
                        depends: depends.clone(),
                        progress: s.build_step_progress(),
                        files_added: Arc::new(Mutex::new(IndexSet::new())),
                        jobs_remaining: Arc::new(Mutex::new(0)),
                        jobs_total: Mutex::new(0),
                        status: Default::default(),
                    }),
                )
            })
            .collect();
        *ctx.hook_ctx.files_in_contention.lock().unwrap() = self.files_in_contention(&ctx)?;
        if self.steps.values().any(|j| j.check_first) {
        } else {
            *ctx.hook_ctx.files_in_contention.lock().unwrap() = Default::default();
        }
        for (_, step) in self.steps.clone() {
            let semaphore = ctx.hook_ctx.try_semaphore();
            let step_ctx = ctx
                .hook_ctx
                .step_contexts
                .lock()
                .unwrap()
                .get(&step.name)
                .unwrap()
                .clone();
            set.spawn({
                let step_ctx = step_ctx.clone();
                let hook_ctx = ctx.hook_ctx.clone();
                async move {
                    let result = step.run_all_jobs(step_ctx.clone(), semaphore).await;
                    if let Err(err) = &result {
                        step_ctx.status_errored(&err.to_string());
                    }
                    hook_ctx
                        .step_contexts
                        .lock()
                        .unwrap()
                        .shift_remove(&step.name);
                    result
                }
            });
        }
        let mut result = Ok(());
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    if ctx.fail_fast {
                        ctx.hook_ctx.failed.cancel();
                        // Mark remaining steps as aborted
                        for step_ctx in ctx.hook_ctx.step_contexts.lock().unwrap().values() {
                            step_ctx.status_aborted();
                        }
                        return Err(err);
                    } else if result.is_ok() {
                        result = Err(err);
                    } else {
                        result = result.wrap_err(err);
                    }
                }
                Err(e) => {
                    std::panic::resume_unwind(e.into_panic());
                }
            }
        }
        if let Some(progress) = ctx.progress {
            if result.is_ok() {
                progress.set_status(ProgressStatus::Done);
            } else {
                progress.set_status(ProgressStatus::Failed);
            }
        }
        result
    }

    fn files_in_contention(&self, ctx: &StepGroupContext) -> Result<HashSet<PathBuf>> {
        if ctx.hook_ctx.run_type != RunType::Fix || !self.steps.values().any(|j| j.check_first) {
            return Ok(Default::default());
        }
        let files = ctx.hook_ctx.files();
        let step_map: HashMap<&str, &Step> = self
            .steps
            .values()
            .map(|step| (step.name.as_str(), step))
            .collect();
        let files_by_step: HashMap<&str, Vec<PathBuf>> = self
            .steps
            .values()
            .map(|step| {
                let step_files = if let Some(pattern) = &step.glob {
                    // Use get_pattern_matches which handles dir filtering internally
                    glob::get_pattern_matches(pattern, &files, step.dir.as_deref())?
                } else if let Some(dir) = &step.dir {
                    // If dir is set without glob, filter files to that directory
                    files
                        .iter()
                        .filter(|f| f.starts_with(dir))
                        .cloned()
                        .collect()
                } else {
                    // No dir and no glob, use all files
                    files.clone()
                };

                Ok((step.name.as_str(), step_files))
            })
            .collect::<Result<_>>()?;
        let mut steps_per_file: HashMap<&Path, Vec<&Step>> = Default::default();
        for (step_name, files) in files_by_step.iter() {
            for file in files {
                let step = step_map.get(step_name).unwrap();
                steps_per_file.entry(file.as_path()).or_default().push(step);
            }
        }

        let mut files_in_contention = HashSet::new();
        for (file, steps) in steps_per_file.iter() {
            if steps.len() > 1 && steps.iter().any(|step| step.fix.is_some()) {
                files_in_contention.insert(file.to_path_buf());
            }
        }

        Ok(files_in_contention)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_inherits_group_fields_without_merging_child_overrides() {
        let group_shell: Script = "bash -o errexit -c".parse().unwrap();
        let child_shell: Script = "zsh -o errexit -c".parse().unwrap();
        let group_exclude = Pattern::Globs(vec!["**/*.snap".to_string()]);
        let child_exclude = Pattern::Globs(vec!["**/*.fixture.js".to_string()]);

        let mut inherited_step = Step::default();
        inherited_step.check = Some("echo inherited".parse().unwrap());
        inherited_step.exclude = Some(Pattern::Globs(vec![]));

        let mut override_step = Step::default();
        override_step.check = Some("echo override".parse().unwrap());
        override_step.dir = Some("different/path".to_string());
        override_step.prefix = Some("npm exec --".to_string());
        override_step.workspace_indicator = Some("eslint.config.js".to_string());
        override_step.shell = Some(child_shell.clone());
        override_step.stage = Some(vec!["eslint-output/**".to_string()]);
        override_step.exclude = Some(child_exclude.clone());

        let mut group = StepGroup {
            dir: Some("packages/frontend".to_string()),
            prefix: Some("mise x --".to_string()),
            workspace_indicator: Some("package.json".to_string()),
            shell: Some(group_shell.clone()),
            stage: Some(vec!["dist/**".to_string()]),
            exclude: Some(group_exclude.clone()),
            steps: IndexMap::from([
                ("prettier".to_string(), inherited_step),
                ("eslint".to_string(), override_step),
            ]),
            ..Default::default()
        };

        group.init("frontend").unwrap();

        let prettier = group.steps.get("prettier").unwrap();
        assert_eq!(prettier.dir.as_deref(), Some("packages/frontend"));
        assert_eq!(prettier.prefix.as_deref(), Some("mise x --"));
        assert_eq!(
            prettier.workspace_indicator.as_deref(),
            Some("package.json")
        );
        assert_eq!(prettier.shell.as_ref(), Some(&group_shell));
        assert_eq!(
            prettier.stage.as_deref(),
            Some(&["dist/**".to_string()][..])
        );
        assert_eq!(prettier.exclude.as_ref(), Some(&group_exclude));

        let eslint = group.steps.get("eslint").unwrap();
        assert_eq!(eslint.dir.as_deref(), Some("different/path"));
        assert_eq!(eslint.prefix.as_deref(), Some("npm exec --"));
        assert_eq!(
            eslint.workspace_indicator.as_deref(),
            Some("eslint.config.js")
        );
        assert_eq!(eslint.shell.as_ref(), Some(&child_shell));
        assert_eq!(
            eslint.stage.as_deref(),
            Some(&["eslint-output/**".to_string()][..])
        );
        assert_eq!(eslint.exclude.as_ref(), Some(&child_exclude));
    }
}
