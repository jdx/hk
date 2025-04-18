use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};

use crate::{Result, glob, step::RunType, step_context::StepContext, step_depends::StepDepends};
use crate::{hook::HookContext, step::Step};

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct StepGroup {
    pub name: Option<String>,
    pub steps: Vec<Arc<Step>>,
}

pub struct StepGroupContext {
    pub hook_ctx: Arc<HookContext>,
    pub progress: Option<Arc<ProgressJob>>,
}

impl StepGroupContext {
    pub fn new(hook_ctx: Arc<HookContext>) -> Self {
        Self {
            hook_ctx,
            progress: None,
        }
    }
    pub fn with_progress(mut self, progress: Arc<ProgressJob>) -> Self {
        self.progress = Some(progress);
        self
    }
}

impl StepGroup {
    pub fn build_all(steps: &[Arc<Step>]) -> Vec<Self> {
        steps
            .iter()
            .fold(vec![], |mut groups, step| {
                if step.exclusive || groups.is_empty() {
                    groups.push(vec![]);
                }
                groups.last_mut().unwrap().push(step.clone());
                if step.exclusive {
                    groups.push(vec![]);
                }
                groups
            })
            .into_iter()
            .filter(|steps| !steps.is_empty())
            .map(|steps| Self { name: None, steps })
            .collect()
    }

    pub fn build_group_progress(&self, name: &str) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body("group: {{group}}")
            .prop("group", &name)
            .start()
    }

    pub async fn run(self, ctx: StepGroupContext) -> Result<()> {
        let depends = Arc::new(StepDepends::new(
            &self
                .steps
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
        ));
        let mut set = tokio::task::JoinSet::new();
        *ctx.hook_ctx.step_contexts.lock().unwrap() = self
            .steps
            .iter()
            .map(|s| {
                (
                    s.name.clone(),
                    Arc::new(StepContext {
                        step: s.clone(),
                        hook_ctx: ctx.hook_ctx.clone(),
                        depends: depends.clone(),
                        progress: s.build_step_progress(),
                        files_added: Arc::new(std::sync::Mutex::new(0)),
                        jobs_remaining: Arc::new(std::sync::Mutex::new(0)),
                        jobs_total: std::sync::Mutex::new(0),
                        status: Default::default(),
                    }),
                )
            })
            .collect();
        *ctx.hook_ctx.files_in_contention.lock().unwrap() = self.files_in_contention(&ctx)?;
        if self.steps.iter().any(|j| j.check_first) {
        } else {
            *ctx.hook_ctx.files_in_contention.lock().unwrap() = Default::default();
        }
        for step in self.steps {
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
        while let Some(res) = set.join_next().await {
            ctx.hook_ctx.inc_completed_jobs(1);
            match res {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    ctx.hook_ctx.failed.cancel();
                    return Err(err);
                }
                Err(e) => {
                    std::panic::resume_unwind(e.into_panic());
                }
            }
        }
        if let Some(progress) = ctx.progress {
            progress.set_status(ProgressStatus::Done);
        }
        Ok(())
    }

    fn files_in_contention(&self, ctx: &StepGroupContext) -> Result<HashSet<PathBuf>> {
        if ctx.hook_ctx.run_type != RunType::Fix || !self.steps.iter().any(|j| j.check_first) {
            return Ok(Default::default());
        }
        let files = ctx.hook_ctx.files();
        let step_map: HashMap<&str, &Step> = self
            .steps
            .iter()
            .map(|step| (step.name.as_str(), &**step))
            .collect();
        let files_by_step: HashMap<&str, Vec<PathBuf>> = self
            .steps
            .iter()
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
