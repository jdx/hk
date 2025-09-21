use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use eyre::Context;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::{
    Result, glob, hook::StepOrGroup, step::RunType, step_context::StepContext,
    step_depends::StepDepends,
};
use crate::{hook::HookContext, step::Step};

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct StepGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,
    pub name: Option<String>,
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
    pub fn init(&mut self, name: &str) {
        self.name = Some(name.to_string());
        for (step_name, step) in self.steps.iter_mut() {
            step.init(step_name);
        }
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
            })
            .collect()
    }

    pub fn build_group_progress(&self, name: &str) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body("group: {{group}}")
            .prop("group", &name)
            .start()
    }

    pub async fn plan(self) -> Result<()> {
        for step_name in self.steps.keys() {
            info!("step: {step_name} –");
            todo!("list files and run types like check-first");
        }
        Ok(())
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
                let files = glob::get_matches(step.glob.as_ref().unwrap_or(&vec![]), &files)?;
                Ok((step.name.as_str(), files))
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
