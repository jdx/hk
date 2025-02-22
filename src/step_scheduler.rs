use crate::{step::StepResponse, tera};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use miette::{IntoDiagnostic, miette};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{
    Mutex, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, OwnedSemaphorePermit, RwLock, Semaphore,
};
use tokio::task::JoinSet;

use crate::{Result, step::StepContext};
use crate::{
    config::Config,
    env,
    git::Git,
    glob,
    settings::Settings,
    step::{CheckType, RunType, Step},
};

pub struct StepScheduler<'a> {
    run_type: RunType,
    repo: &'a Git,
    steps: Vec<Step>,
    files: IndexSet<PathBuf>,
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
            steps: hook
                .values()
                .flat_map(|s| match &s.r#type {
                    Some(r#type) if r#type == "check" || r#type == "fix" => config
                        .linters
                        .values()
                        .cloned()
                        .map(|linter| linter.into())
                        .collect(),
                    _ => vec![s.clone()],
                })
                .collect(),
            files: Default::default(),
            failed: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(settings.jobs().get())),
            file_locks: Default::default(),
        }
    }

    pub fn with_files(mut self, files: IndexSet<PathBuf>) -> Self {
        self.files = files;
        self
    }

    pub async fn run(self) -> Result<()> {
        let check_files_globs = self.steps.iter().flat_map(|s| &s.check_lock_globs).flatten().collect_vec();
        let fix_files_globs = self.steps.iter().flat_map(|s| &s.fix_lock_globs).flatten().collect_vec();
        *self.file_locks.lock().await = self
            .files
            .iter()
            .cloned()
            .chain(self.repo.all_files_matching_globs(&check_files_globs.iter().chain(fix_files_globs.iter()).map(|g| g.as_str()).collect_vec())?)
            .map(|file| (file, Arc::new(RwLock::new(()))))
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
        let files_vec = self.files.iter().collect_vec();

        for group in groups {
            let mut set = JoinSet::new();
            let depend_locks: IndexMap<String, _> = group
                .iter()
                .map(|s| (s.name.clone(), Arc::new(RwLock::new(()))))
                .collect();
            // TODO: add check_lock_globs and fix_lock_globs to files_in_contention
            let files_in_contention = Arc::new(self.files_in_contention(&group, &files_vec)?);

            // Spawn all tasks
            for step in &group {
                let Some(run_type) = step.available_run_type(self.run_type) else {
                    debug!("{step}: skipping step due to no available run type");
                    continue;
                };
                let mut files = if let Some(glob) = &step.glob {
                    let matches = glob::get_matches(glob, &files_vec)?;
                    if matches.is_empty() {
                        debug!("{step}: no matches for step");
                        continue;
                    }
                    matches
                } else {
                    files_vec.iter().map(|f| f.to_path_buf()).collect_vec()
                };
                if let Some(dir) = &step.dir {
                    files.retain(|f| f.starts_with(dir));
                    if files.is_empty() {
                        debug!("{step}: no matches for step in {dir}");
                        continue;
                    }
                }
                let ctx = StepContext {
                    run_type,
                    check_locks: self
                        .file_locks
                        .lock()
                        .await
                        .iter()
                        .filter(|(k, _)| files.contains(k))
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    files,
                    depend_self: Some(
                        depend_locks
                            .get(&step.name)
                            .unwrap()
                            .clone()
                            .write_owned()
                            .await,
                    ),
                };

                let depends = step
                    .depends
                    .iter()
                    .map(|dep| {
                        depend_locks
                            .iter()
                            .find(|(name, _)| *name == dep)
                            .map(|(name, lock)| (name.to_string(), lock.clone()))
                            .ok_or_else(|| miette!("No step named {dep} found in group"))
                    })
                    .collect::<Result<IndexMap<String, _>>>()?;

                if !step.is_profile_enabled() {
                    continue;
                }

                let tctx = tera::Context::default();
                if let Some(workspaces) = step.workspaces_for_files(&ctx.files)? {
                    let step = (*step).clone();
                    let semaphore = self.semaphore.clone();
                    let failed = self.failed.clone();
                    let files_in_contention = files_in_contention.clone();
                    set.spawn(async move {
                        let mut rsp = StepResponse::default();
                        let mut set = JoinSet::new();
                        for workspace_file in workspaces {
                            let mut tctx = tctx.clone();
                            tctx.with_workspace_file(&workspace_file);
                            StepScheduler::run_step(
                                semaphore.clone(),
                                failed.clone(),
                                ctx.clone(),
                                tctx,
                                depends.clone(),
                                &step,
                                &mut set,
                                files_in_contention.clone(),
                            )
                            .await?;
                        }
                        // TODO: abort on first error
                        while let Some(result) = set.join_next().await {
                            match result {
                                Ok(Ok(r)) => rsp.extend(r),
                                Ok(Err(e)) => return Err(e),
                                Err(e) => return Err(e).into_diagnostic(),
                            }
                        }
                        Ok(rsp)
                    });
                } else {
                    StepScheduler::run_step(
                        self.semaphore.clone(),
                        self.failed.clone(),
                        ctx,
                        tctx,
                        depends,
                        step,
                        &mut set,
                        files_in_contention.clone(),
                    )
                    .await?;
                }
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

    fn files_in_contention<P: AsRef<Path>>(&self, steps: &[&Step], files: &[P]) -> Result<HashSet<PathBuf>> {
        // TODO: add check_lock_globs and fix_lock_globs to files_in_contention
        let step_map: IndexMap<String, &Step> = steps
            .iter()
            .map(|step| (step.name.clone(), *step))
            .collect();
        let files_by_step: HashMap<String, Vec<PathBuf>> = steps
            .iter()
            .map(|step| {
                let files = glob::get_matches(step.glob.as_ref().unwrap_or(&vec![]), files)?;
                Ok((step.name.clone(), files))
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
            if steps
                .iter()
                .any(|step| step.available_run_type(self.run_type) == Some(RunType::Fix))
            {
                files_in_contention.insert(file.to_path_buf());
            }
        }

        Ok(files_in_contention)
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_step(
        semaphore: Arc<Semaphore>,
        failed: Arc<Mutex<bool>>,
        ctx: StepContext,
        tctx: tera::Context,
        depends: IndexMap<String, Arc<RwLock<()>>>,
        step: &Step,
        set: &mut JoinSet<Result<StepResponse>>,
        files_in_contention: Arc<HashSet<PathBuf>>,
    ) -> Result<()> {
        let permit = semaphore.clone().acquire_owned().await.into_diagnostic()?;

        trace!("{step}: spawning step");
        let step = step.clone();
        set.spawn(async move {
            let depends = depends;
            if *env::HK_CHECK_FIRST
                && step.check_first
                && matches!(ctx.run_type, RunType::Fix)
                && ctx
                    .files
                    .iter()
                    .any(|file| files_in_contention.contains(file))
            {
                let mut check_ctx = ctx.clone();
                check_ctx.run_type = match ctx.run_type {
                    RunType::Fix if step.check_diff.is_some() => RunType::Check(CheckType::Diff),
                    RunType::Fix if step.check_list_files.is_some() => {
                        RunType::Check(CheckType::ListFiles)
                    }
                    RunType::Fix => RunType::Check(CheckType::Check),
                    _ => unreachable!(),
                };
                debug!("{step}: running check step first due to fix step contention");
                match run(
                    check_ctx,
                    tctx.clone(),
                    semaphore.clone(),
                    &step,
                    failed.clone(),
                    permit,
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
            let permit = semaphore.clone().acquire_owned().await.into_diagnostic()?;
            match run(
                ctx,
                tctx,
                semaphore,
                &step,
                failed.clone(),
                permit,
                &depends,
            )
            .await
            {
                Ok(rsp) => Ok(rsp),
                Err(err) => {
                    // Mark as failed to prevent new steps from starting
                    *failed.lock().await = true;
                    Err(err)
                }
            }
        });
        Ok(())
    }
}

async fn run(
    ctx: StepContext,
    tctx: tera::Context,
    semaphore: Arc<Semaphore>,
    step: &Step,
    failed: Arc<Mutex<bool>>,
    permit: OwnedSemaphorePermit,
    depends: &IndexMap<String, Arc<RwLock<()>>>,
) -> Result<StepResponse> {
    let _locks = StepLocks::lock(step, &ctx, semaphore, permit, depends).await?;
    if *failed.lock().await {
        trace!("{step}: skipping step due to previous failure");
        return Ok(Default::default());
    }
    match step.run(ctx, tctx).await {
        Ok(rsp) => {
            trace!("{step}: successfully ran step");
            Ok(rsp)
        }
        Err(err) => {
            trace!("{step}: failed to run step: {err}");
            Err(err.wrap_err(step.name.clone()))
        }
    }
}

#[allow(unused)]
struct StepLocks {
    read_flocks: Vec<OwnedRwLockReadGuard<()>>,
    write_flocks: Vec<OwnedRwLockWriteGuard<()>>,
    permit: OwnedSemaphorePermit,
}

impl StepLocks {
    fn try_lock(
        step: &Step,
        ctx: &StepContext,
        depends: &IndexMap<String, Arc<RwLock<()>>>,
        permit: OwnedSemaphorePermit,
    ) -> Option<Self> {
        let mut read_flocks = vec![];
        let mut write_flocks = vec![];
        for (name, depends) in depends.iter() {
            if depends.try_read().is_err() {
                trace!("{step}: waiting for {name} to finish");
                return None;
            }
        }
        for (path, lock) in ctx.file_locks.iter() {
            let lock = lock.clone();
            match (step.stomp, ctx.run_type) {
                (_, RunType::Run) => {}
                (true, _) | (_, RunType::Check(_)) => match lock.clone().try_read_owned() {
                    Ok(lock) => read_flocks.push(lock),
                    Err(_) => {
                        trace!("{step}: waiting for {} to finish", path.display());
                        return None;
                    }
                },
                (_, RunType::Fix) => match lock.clone().try_write_owned() {
                    Ok(lock) => write_flocks.push(lock),
                    Err(_) => {
                        trace!("{step}: waiting for {} to finish", path.display());
                        return None;
                    }
                },
            }
        }
        Some(StepLocks {
            read_flocks,
            write_flocks,
            permit,
        })
    }

    async fn lock(
        step: &Step,
        ctx: &StepContext,
        semaphore: Arc<Semaphore>,
        permit: OwnedSemaphorePermit,
        depends: &IndexMap<String, Arc<RwLock<()>>>,
    ) -> Result<Self> {
        if let Some(locks) = Self::try_lock(step, ctx, depends, permit) {
            return Ok(locks);
        }
        let mut read_flocks = vec![];
        let mut write_flocks = vec![];
        for (_name, depends) in depends.iter() {
            read_flocks.push(depends.clone().read_owned().await);
        }
        for (_path, lock) in ctx.file_locks.iter() {
            let lock = lock.clone();
            match (step.stomp, ctx.run_type) {
                (_, RunType::Run) => {}
                (true, _) | (_, RunType::Check(_)) => {
                    read_flocks.push(lock.clone().read_owned().await)
                }
                (_, RunType::Fix) => write_flocks.push(lock.clone().write_owned().await),
            }
        }
        Ok(Self {
            read_flocks,
            write_flocks,
            permit: semaphore.clone().acquire_owned().await.into_diagnostic()?,
        })
    }
}
