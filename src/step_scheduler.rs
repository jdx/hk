use crate::{
    env,
    error::Error,
    hook::Hook,
    step_depends::StepDepends,
    step_job::{StepJob, StepJobStatus},
    step_locks::StepLocks,
    step_queue::StepQueueBuilder,
    step_response::StepResponse,
    tera::Context,
    ui::style,
};
use clx::progress::{self, ProgressJobBuilder, ProgressOutput};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    panic,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::task::JoinSet;

use crate::{Result, step_context::StepContext};
use crate::{
    git::Git,
    settings::Settings,
    step::{CheckType, RunType},
};

pub struct StepScheduler {
    run_type: RunType,
    repo: Arc<Mutex<Git>>,
    hook: Hook,
    files: Vec<PathBuf>,
    tctx: Context,
    failed: Arc<Mutex<bool>>,
    semaphore: Arc<Semaphore>,
}

impl StepScheduler {
    pub fn new(hook: &Hook, run_type: RunType, repo: Arc<Mutex<Git>>) -> Self {
        let settings = Settings::get();
        Self {
            run_type,
            repo,
            hook: hook.clone(),
            files: vec![],
            tctx: Default::default(),
            failed: Arc::new(Mutex::new(false)),
            semaphore: Arc::new(Semaphore::new(settings.jobs().get())),
        }
    }

    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.files = files;
        self
    }

    pub fn with_linters(mut self, linters: &[String]) -> Self {
        if linters.is_empty() {
            return self;
        }
        self.hook.steps.retain(|name, _| linters.contains(name));
        self
    }

    pub fn with_tctx(mut self, tctx: Context) -> Self {
        self.tctx = tctx;
        self
    }

    pub async fn run(self) -> Result<()> {
        let settings = Settings::get();
        let jobs = settings.jobs().get();
        let file_locks = self
            .files
            .iter()
            .map(|file| (file.clone(), Arc::new(RwLock::new(()))))
            .collect::<IndexMap<PathBuf, _>>();
        let steps = self
            .hook
            .steps
            .into_iter()
            .map(|(_, step)| Arc::new(step))
            .collect::<Vec<_>>();
        let queue = StepQueueBuilder::new(steps, self.files, self.run_type).build()?;
        let total_jobs = queue.groups.iter().flatten().count();
        let mut remaining_jobs = total_jobs;

        for group in queue.groups.iter() {
            let mut set = JoinSet::new();
            let step_contexts: HashMap<String, Arc<StepContext>> = group
                .iter()
                .map(|job| job.step.clone())
                .unique_by(|step| step.name.clone())
                .map(|step| {
                    let jobs_total = group
                        .iter()
                        .filter(|job| job.step.name == step.name)
                        .count();
                    (
                        step.name.clone(),
                        Arc::new(StepContext {
                            semaphore: self.semaphore.clone(),
                            failed: self.failed.clone(),
                            file_locks: file_locks.clone(),
                            tctx: self.tctx.clone(),
                            depends: Arc::new(StepDepends::new(group)),
                            progress: step.build_step_progress(),
                            files_added: Arc::new(std::sync::Mutex::new(0)),
                            jobs_total,
                            jobs_remaining: Arc::new(std::sync::Mutex::new(jobs_total)),
                        }),
                    )
                })
                .collect();

            let total_progress = if progress::output() == ProgressOutput::Text || total_jobs <= jobs
            {
                None
            } else {
                Some(
                    ProgressJobBuilder::new()
                        .progress_total(total_jobs)
                        .progress_current(total_jobs - remaining_jobs)
                        .body(vec!["{{progress_bar()}}".to_string()])
                        .start(),
                )
            };

            for job in group {
                StepScheduler::run_step(
                    step_contexts.get(&job.step.name).unwrap().clone(),
                    job.clone(),
                    &mut set,
                )
                .await?;
            }

            // Wait for tasks and abort on first error
            let mut files_to_stage = IndexSet::new();
            let abort = async |set: &mut JoinSet<Result<StepResponse>>, e: eyre::Error| {
                set.abort_all();
                for p in step_contexts.values().map(|ctx| &ctx.progress) {
                    if p.is_running() {
                        p.set_status(clx::progress::ProgressStatus::DoneCustom(
                            style::eyellow("▲").to_string(),
                        ));
                    }
                    for child in p.children() {
                        if child.is_running() {
                            child.set_status(clx::progress::ProgressStatus::DoneCustom(
                                style::eyellow("▲").to_string(),
                            ));
                        }
                    }
                }
                progress::flush();
                if !log::log_enabled!(log::Level::Debug) {
                    if let Some(ensembler::Error::ScriptFailed(err)) =
                        e.chain().find_map(|e| e.downcast_ref::<ensembler::Error>())
                    {
                        if let Err(err) = self.repo.lock().await.pop_stash() {
                            warn!("Failed to pop stash: {:?}", err);
                        }
                        clx::progress::flush();
                        let bin = &err.0;
                        let args = &err.1;
                        let output = &err.2;
                        let result = &err.3;
                        let mut cmd = format!("{} {}", bin, args.join(" "));
                        if cmd.starts_with("sh -o errexit -c ") {
                            cmd = cmd[17..].to_string();
                        }
                        eprintln!("{}\n{output}", style::ered(format!("Error running {cmd}")));
                        if let Err(e) = write_output_file(result) {
                            eprintln!("Error writing output file: {e:?}");
                        }
                        std::process::exit(1);
                    }
                }
                Err(e)
            };
            while let Some(result) = set.join_next().await {
                remaining_jobs -= 1;
                if let Some(total_progress) = &total_progress {
                    total_progress.progress_current(total_jobs - remaining_jobs);
                }
                match result {
                    Ok(Ok(ctx)) => {
                        files_to_stage.extend(ctx.files_to_add);
                    }
                    Ok(Err(e)) => {
                        // Task failed to execute
                        return abort(&mut set, e).await;
                    }
                    Err(e) => {
                        // JoinError occurred
                        return abort(&mut set, eyre::eyre!("join error: {e:?}")).await;
                    }
                }
            }
            if let Some(total_progress) = &total_progress {
                total_progress.remove();
            }

            if !files_to_stage.is_empty() {
                trace!("staging files: {:?}", &files_to_stage);
                self.repo.lock().await.add(
                    &files_to_stage
                        .iter()
                        .map(|f| f.to_str().unwrap())
                        .collect_vec(),
                )?;
            }
        }
        Ok(())
    }

    async fn run_step(
        ctx: Arc<StepContext>,
        mut job: StepJob,
        set: &mut JoinSet<Result<StepResponse>>,
    ) -> Result<()> {
        let step = job.step.clone();

        trace!("{step}: spawning step");
        set.spawn(async move {
            panic::set_hook(Box::new(|info| {
                error!("panic: {info}");
            }));
            if job.check_first {
                let mut check_job = job.clone();
                check_job.run_type = match (&*step, job.run_type) {
                    (step, RunType::Fix) if step.check_diff.is_some() => RunType::Check(CheckType::Diff),
                    (step, RunType::Fix) if step.check_list_files.is_some() => {
                        RunType::Check(CheckType::ListFiles)
                    }
                    (_step, RunType::Fix) => RunType::Check(CheckType::Check),
                    _ => unreachable!(),
                };
                debug!("{step}: running check step first due to fix step contention");
                match run(
                    &ctx,
                    check_job,
                )
                .await
                {
                    Ok(rsp) => {
                        debug!("{step}: successfully ran check step first");
                        ctx.depends.job_done(&step.name);
                        return Ok(rsp);
                    }
                    Err(e) => {
                        if let Some(Error::CheckListFailed { source, stdout }) =
                            e.downcast_ref::<Error>()
                        {
                            debug!("{step}: failed check step first: {source}");
                            let filtered_files: HashSet<PathBuf> =
                                stdout.lines().map(|p| match PathBuf::from(p).canonicalize() {
                                    Ok(p) => p,
                                    Err(e) => {
                                        warn!("{step}: failed to canonicalize file: {e}");
                                        PathBuf::from(p)
                                    }
                                }).collect();
                            let files: IndexSet<PathBuf> = job.files.into_iter().filter(|f| {
                                let f = match f.canonicalize() {
                                    Ok(p) => p,
                                    Err(e) => {
                                        warn!("{step}: failed to canonicalize file: {e}");
                                        f.to_path_buf()
                                    }
                                };
                                filtered_files.contains(&f)
                            }).collect();
                            for f in filtered_files.into_iter().filter(|f| !files.contains(f)) {
                                warn!("{step}: file in check_list_files not found in original files: {}", f.display());
                            }
                            job.files = files.into_iter().collect();
                        }
                        debug!("{step}: failed check step first: {e}");
                    }
                }
            }
            let failed = ctx.failed.clone();
            let result = match run(
                &ctx,
                job,
            )
            .await
            {
                Ok(rsp) => Ok(rsp),
                Err(err) => {
                    // Mark as failed to prevent new steps from starting
                    *failed.lock().await = true;
                    Err(err)
                }
            };
            ctx.depends.job_done(&step.name);
            result
        });
        Ok(())
    }
}

fn write_output_file(result: &ensembler::CmdResult) -> Result<()> {
    let path = env::HK_STATE_DIR.join("output.log");
    std::fs::create_dir_all(path.parent().unwrap())?;
    let output = console::strip_ansi_codes(&result.combined_output);
    std::fs::write(&path, output.to_string())?;
    eprintln!("\nSee {} for full command output", path.display());
    Ok(())
}

async fn run(ctx: &StepContext, mut job: StepJob) -> Result<StepResponse> {
    let step = job.step.clone();
    match job.status {
        StepJobStatus::Pending => {
            let locks = StepLocks::lock(ctx, &job).await?;
            job.status = StepJobStatus::Started(locks);
        }
        // StepJobStatus::Ready(locks) => {
        //     job.status = StepJobStatus::Started(locks);
        // }
        status => unreachable!("invalid status: {}", status),
    }
    if *ctx.failed.lock().await {
        trace!("{step}: skipping step due to previous failure");
        return Ok(Default::default());
    }
    match step.run(ctx, &job).await {
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
