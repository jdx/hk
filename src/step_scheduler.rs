use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use miette::{miette, IntoDiagnostic};
use std::{collections::HashSet, iter::once, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, OwnedSemaphorePermit, RwLock, Semaphore};
use tokio::task::JoinSet;

use crate::{
    config::Config, env, git::Git, glob, settings::Settings, step::{RunType, Step}
};
use crate::{step::StepContext, Result};

pub struct StepScheduler<'a> {
    run_type: RunType,
    repo: &'a Git,
    steps: Vec<Step>,
    files: Vec<PathBuf>,
    failed: Arc<Mutex<bool>>,
    semaphore: Arc<Semaphore>,
    file_locks: Mutex<IndexMap<PathBuf, Arc<RwLock<()>>>>,
}

impl<'a> StepScheduler<'a> {
    pub fn new(hook: &IndexMap<String, Step>, run_type: RunType, repo: &'a Git) -> Self {
        let settings = Settings::get();
        let config = Config::get().unwrap();
        Self {
            run_type,
            repo,
            steps: hook.values().flat_map(|s| match &s.r#type {
                Some(r#type) if r#type == "check" || r#type == "fix" => {
                    config.linters.values().cloned().map(|linter| linter.into()).collect()
                }
                _ => vec![s.clone()],
            }).collect(),
            files: vec![],
            failed: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(settings.jobs().get())),
            file_locks: Default::default(),
        }
    }

    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.files = files;
        self
    }

    pub async fn run(self) -> Result<()> {
        *self.file_locks.lock().await = self
            .files
            .iter()
            .map(|file| (file.clone(), Arc::new(RwLock::new(()))))
            .collect();
        // groups is a list of list of steps which are separated by exclusive steps
        // any exclusive step will be in a group by itself
        let groups = self.steps.iter().fold(vec![], |mut groups, step| {
            if step.exclusive || groups.is_empty() {
                groups.push(vec![]);
            }
            groups.last_mut().unwrap().push(step);
            groups
        });

        for group in groups {
            let mut set = JoinSet::new();
            let depend_locks: IndexMap<String, _> = group
                .iter()
                .map(|s| (s.name.clone(), Arc::new(RwLock::new(()))))
                .collect();
            let fix_steps_in_contention =
                Arc::new(self.fix_steps_in_contention(&group, &self.files)?);

            // Spawn all tasks
            for step in &group {
                let Some(run_type) = step.available_run_type(self.run_type) else {
                    debug!("{step}: skipping step due to no available run type");
                    continue;
                };
                let files = if let Some(glob) = &step.glob {
                    let matches = glob::get_matches(glob, &self.files)?;
                    if matches.is_empty() {
                        debug!("{step}: no matches for step");
                        continue;
                    }
                    matches
                } else {
                    self.files.clone()
                };
                let ctx = StepContext {
                    run_type,
                    files_to_add: vec![],
                    file_locks: self
                        .file_locks
                        .lock()
                        .await
                        .iter()
                        .filter(|(k, _)| files.contains(k))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    files,
                };

                let depends = step
                    .depends
                    .iter()
                    .chain(once(&step.name))
                    .map(|dep| {
                        depend_locks
                            .iter()
                            .find(|(name, _)| *name == dep || **name == step.name)
                            .map(|(name, lock)| (name.to_string(), lock.clone()))
                            .ok_or_else(|| miette!("No step named {dep} found in group"))
                    })
                    .collect::<Result<IndexMap<String, _>>>()?;

                self.run_step(
                    depends,
                    step,
                    &mut set,
                    ctx,
                    fix_steps_in_contention.clone(),
                )
                .await?;
            }

            // Wait for tasks and abort on first error
            let mut files_to_stage = IndexSet::new();
            while let Some(result) = set.join_next().await {
                match result {
                    Ok(Ok(ctx)) => {
                        files_to_stage.extend(ctx.files_to_add);
                    }
                    Ok(Err(e)) => {
                        // Task failed to execute
                        set.abort_all();
                        return Err(e);
                    }
                    Err(e) => {
                        // JoinError occurred
                        set.abort_all();
                        return Err(e).into_diagnostic();
                    }
                }
            }

            if !files_to_stage.is_empty() {
                trace!("staging files: {:?}", &files_to_stage);
                self.repo.add(
                    &files_to_stage
                        .iter()
                        .map(|f| f.to_str().unwrap())
                        .collect_vec(),
                )?;
            }
        }
        Ok(())
    }

    /// Returns a subset of steps that have a fix command and need to at least 1 file another step will need a read/write lock on
    fn fix_steps_in_contention(
        &self,
        steps: &[&Step],
        files: &[PathBuf],
    ) -> Result<HashSet<String>> {
        if matches!(self.run_type, RunType::Check) {
            return Ok(Default::default());
        }
        let files_per_step: IndexMap<&Step, HashSet<PathBuf>> = steps
            .iter()
            .map(|step| {
                let files = glob::get_matches(step.glob.as_ref().unwrap_or(&vec![]), files)?;
                Ok((*step, files.into_iter().collect()))
            })
            .collect::<Result<_>>()?;
        let fix_steps = steps
            .iter()
            .copied()
            .filter(|step| {
                matches!(
                    step.available_run_type(self.run_type),
                    Some(RunType::Fix)
                )
            })
            .filter(|step| {
                let other_files = files_per_step
                    .iter()
                    .filter(|(k, _)| *k != step)
                    .map(|(_, v)| v)
                    .collect::<Vec<_>>();
                files_per_step.get(*step).unwrap().iter().any(|file| {
                    other_files
                        .iter()
                        .any(|other_files| other_files.contains(file))
                })
            })
            .map(|step| step.name.clone())
            .collect();
        Ok(fix_steps)
    }

    async fn run_step(
        &self,
        mut depends: IndexMap<String, Arc<RwLock<()>>>,
        step: &Step,
        set: &mut JoinSet<Result<StepContext>>,
        ctx: StepContext,
        fix_steps_in_contention: Arc<HashSet<String>>,
    ) -> Result<()> {
        let settings = Settings::get();
        let semaphore = self.semaphore.clone();
        let permit = semaphore.clone().acquire_owned().await.into_diagnostic()?;
        let failed = self.failed.clone();
        if *failed.lock().await {
            trace!("{step}: skipping step due to previous failure");
            return Ok(());
        }
        // Check if step should be skipped based on HK_SKIP_STEPS
        if crate::env::HK_SKIP_STEPS.contains(&step.name) {
            warn!("{step}: skipping step due to HK_SKIP_STEPS");
            return Ok(());
        }
        if let Some(profiles) = step.enabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let missing_profiles = profiles.difference(&enabled_profiles).collect::<Vec<_>>();
            if !missing_profiles.is_empty() {
                trace!(
                    "{step}: skipping step due to missing profile: {}",
                    missing_profiles.iter().join(", ")
                );
                return Ok(());
            }
        }
        if let Some(profiles) = step.disabled_profiles() {
            let enabled_profiles = settings.enabled_profiles();
            let disabled_profiles = profiles.intersection(&enabled_profiles).collect::<Vec<_>>();
            if !disabled_profiles.is_empty() {
                trace!(
                    "{step}: skipping step due to disabled profile: {}",
                    disabled_profiles.iter().join(", ")
                );
                return Ok(());
            }
        }
        let depend_self = depends.shift_remove(&step.name).unwrap();
        let self_depend_write_lock = depend_self.write_owned().await;

        trace!("{step}: spawning step");
        let step = step.clone();
        set.spawn(async move {
            let _self_depend_write_lock = self_depend_write_lock;
            let permit = Arc::new(Mutex::new(Some(permit)));
            let depends = depends;
            if *env::HK_CHECK_FIRST
                && step.check_first
                && matches!(ctx.run_type, RunType::Fix)
                && fix_steps_in_contention.contains(&step.name)
            {
                let mut ctx = ctx.clone();
                ctx.run_type = match ctx.run_type {
                    RunType::Fix => RunType::Check,
                    _ => unreachable!(),
                };
                debug!("{step}: running check step first due to fix step contention");
                match run(
                    ctx,
                    semaphore.clone(),
                    &step,
                    failed.clone(),
                    permit.clone(),
                    &depends,
                )
                .await
                {
                    Ok(ctx) => {
                        debug!("{step}: successfully ran check step first");
                        return Ok(ctx);
                    }
                    Err(e) => {
                        warn!("{step}: failed check step first: {e}");
                    }
                }
            }
            return run(ctx, semaphore, &step, failed, permit, &depends).await;
        });
        Ok(())
    }
}

async fn run(
    ctx: StepContext,
    semaphore: Arc<Semaphore>,
    step: &Step,
    failed: Arc<Mutex<bool>>,
    _permit: Arc<Mutex<Option<OwnedSemaphorePermit>>>,
    depends: &IndexMap<String, Arc<RwLock<()>>>,
) -> Result<StepContext> {
    let mut read_flocks = vec![];
    let mut write_flocks = vec![];
    for (path, lock) in ctx.file_locks.iter() {
        let lock = lock.clone();
        match (step.stomp, ctx.run_type) {
            (_, RunType::Run) => {},
            (true, _) | (_, RunType::Check) => {
                match lock.clone().try_read_owned() {
                    Ok(lock) => read_flocks.push(lock),
                    Err(_) => {
                        trace!("{step}: waiting for {} to finish", path.display());
                        *_permit.lock().await = None; // release the permit so someone else can work
                        read_flocks.push(lock.read_owned().await);
                    }
                }
            }
            (_, RunType::Fix) => match lock.clone().try_write_owned() {
                Ok(lock) => write_flocks.push(lock),
                Err(_) => {
                    trace!("{step}: waiting for {} to finish", path.display());
                    // TODO: this has a race condition, the permit is released but we may retain some file locks
                    *_permit.lock().await = None; // release the permit so someone else can work
                    write_flocks.push(lock.write_owned().await);
                }
            },
        }
    }
    for (name, depends) in depends.iter() {
        match depends.clone().try_read_owned() {
            Ok(lock) => read_flocks.push(lock),
            Err(_) => {
                trace!("{step}: waiting for {name} to finish");
                *_permit.lock().await = None; // release the permit so someone else can work
                read_flocks.push(depends.clone().read_owned().await);
            }
        }
    }
    if _permit.lock().await.is_none() {
        *_permit.lock().await = Some(semaphore.acquire_owned().await.into_diagnostic()?);
    }
    match step.run(ctx).await {
        Ok(ctx) => Ok(ctx),
        Err(err) => {
            // Mark as failed to prevent new steps from starting
            *failed.lock().await = true;
            Err(err.wrap_err(step.name.clone()))
        }
    }
}
