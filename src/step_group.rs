use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use eyre::Context;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, PickFirst, serde_as};
use tokio_util::sync::CancellationToken;

use crate::{
    Result,
    hook::{HookContext, StepOrGroup},
    step::{CommandPrefix, Pattern, RunType, Script, SharedBatchJobs, Step},
    step_context::StepContext,
    step_depends::StepDepends,
};

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[serde_as]
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct StepGroup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,
    pub name: Option<String>,
    pub workspace_indicator: Option<String>,
    pub prefix: Option<CommandPrefix>,
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

/// Whether the run was cancelled from outside (Ctrl-C) rather than by a
/// step failure aborting its siblings.
pub(crate) fn cancelled_by_user(hook_ctx: &HookContext) -> bool {
    user_cancelled(&hook_ctx.failed, &hook_ctx.fail_fast_aborted)
}

/// The single place that decides how a step ends once its jobs are joined.
///
/// The cancellation token is authoritative for a user's Ctrl-C, whatever the
/// jobs reported:
/// - jobs all Ok, or an error that is only an allowed failure (which the step
///   group would otherwise turn into success): the step is cancelled, as on
///   a run that was interrupted. `Ok(None)` is returned for a fail-fast abort,
///   which ends quietly because the step that failed reports the error.
/// - any other error, such as a real failure that happened before the
///   Ctrl-C: that failure is surfaced unchanged.
/// - not cancelled: the jobs' result as is, wrapped in `Some`.
pub(crate) fn settle_joined_jobs<T>(
    hook_ctx: &HookContext,
    joined: Result<T>,
    is_allowed: impl Fn(&eyre::Report) -> bool,
) -> Result<Option<T>> {
    settle(
        &hook_ctx.failed,
        &hook_ctx.fail_fast_aborted,
        joined,
        is_allowed,
    )
}

fn settle<T>(
    failed: &CancellationToken,
    aborted: &AtomicBool,
    joined: Result<T>,
    is_allowed: impl Fn(&eyre::Report) -> bool,
) -> Result<Option<T>> {
    let user = user_cancelled(failed, aborted);
    match joined {
        Err(err) if !(user && is_allowed(&err)) => Err(err),
        _ if user => Err(ensembler::Error::Cancelled.into()),
        Ok(value) if !failed.is_cancelled() => Ok(Some(value)),
        Ok(_) => Ok(None),
        Err(err) => Err(err),
    }
}

fn user_cancelled(failed: &CancellationToken, aborted: &AtomicBool) -> bool {
    failed.is_cancelled() && !aborted.load(Ordering::SeqCst)
}

/// Record that a step failure is about to cancel the run, unless something
/// else (Ctrl-C) already did.
fn mark_failure_abort(failed: &CancellationToken, aborted: &AtomicBool) {
    if !failed.is_cancelled() {
        aborted.store(true, Ordering::SeqCst);
    }
}

/// Stop everything that is running after a fail-fast failure.
///
/// Remaining steps are marked before their commands are cancelled so a woken
/// runner cannot record cancellation as a command failure.
///
/// `fail_fast_aborted` is only set when this is the first thing to cancel the
/// run. After a user's Ctrl-C the token is already cancelled, and the
/// cancellation errors that follow must reach the caller exactly as they did
/// before fail-fast aborts existed.
pub(crate) async fn abort_running_steps(hook_ctx: &HookContext) {
    mark_failure_abort(&hook_ctx.failed, &hook_ctx.fail_fast_aborted);
    for step_ctx in hook_ctx.step_contexts.lock().unwrap().values() {
        step_ctx.status_aborted();
    }
    crate::step::cancel_running_steps(&hook_ctx.failed).await;
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
        let steps = self
            .steps
            .values()
            .cloned()
            .map(Arc::new)
            .collect::<Vec<_>>();
        let batch_jobs = Arc::new(SharedBatchJobs::new(steps.clone()));
        *ctx.hook_ctx.step_contexts.lock().unwrap() = self
            .steps
            .values()
            .zip(&steps)
            .map(|(s, shared_step)| {
                (
                    s.name.clone(),
                    Arc::new(StepContext {
                        step: shared_step.clone(),
                        hook_ctx: ctx.hook_ctx.clone(),
                        depends: depends.clone(),
                        batch_jobs: batch_jobs.clone(),
                        progress: s.build_step_progress(),
                        files_added: Arc::new(Mutex::new(IndexSet::new())),
                        jobs_remaining: Arc::new(Mutex::new(0)),
                        jobs_total: Mutex::new(0),
                        status: Default::default(),
                    }),
                )
            })
            .collect();
        *ctx.hook_ctx.files_in_contention.lock().unwrap() =
            self.files_in_contention_for(&ctx.hook_ctx.files(), ctx.hook_ctx.run_type)?;
        if self.steps.values().any(|j| j.check_first()) {
        } else {
            *ctx.hook_ctx.files_in_contention.lock().unwrap() = Default::default();
        }
        for step in steps {
            let semaphore = ctx.hook_ctx.try_semaphore();
            let step_ctx = ctx
                .hook_ctx
                .step_contexts
                .lock()
                .unwrap()
                .get(&step.name)
                .unwrap()
                .clone();
            let fail_fast = ctx.fail_fast;
            set.spawn({
                let step_ctx = step_ctx.clone();
                let hook_ctx = ctx.hook_ctx.clone();
                async move {
                    let result = step
                        .clone()
                        .run_all_jobs(step_ctx.clone(), semaphore, fail_fast)
                        .await;
                    let failure_allowed = match match &result {
                        Err(err) if crate::error::is_command_failure(err) => {
                            step.failure_is_allowed(&hook_ctx.expr_ctx())
                        }
                        _ => Ok(false),
                    } {
                        Ok(failure_allowed) => failure_allowed,
                        Err(err) => {
                            step_ctx.status_errored(&err.to_string());
                            if !fail_fast {
                                step_ctx.depends.mark_done(&step.name)?;
                            }
                            hook_ctx
                                .step_contexts
                                .lock()
                                .unwrap()
                                .shift_remove(&step.name);
                            return Err(err);
                        }
                    };
                    if failure_allowed {
                        hook_ctx.mark_step_failure_allowed(&step.name);
                    }
                    if let Err(err) = &result {
                        step_ctx.status_errored(&err.to_string());
                    }
                    if (!fail_fast || failure_allowed) && result.is_err() {
                        step_ctx.depends.mark_done(&step.name)?;
                    }
                    hook_ctx
                        .step_contexts
                        .lock()
                        .unwrap()
                        .shift_remove(&step.name);
                    if failure_allowed { Ok(()) } else { result }
                }
            });
        }
        let mut result = Ok(());
        // The first cancellation error after a user's Ctrl-C. Steps that were
        // only cancelled (a dependent that never started) can report before the
        // step whose command was killed, so wait for the rest and let a more
        // specific error, or the cancelled command's own, come first.
        let mut cancelled: Option<eyre::Report> = None;
        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    // Once a failure has cancelled the other steps, their
                    // cancellation errors can reach us before the failure
                    // that caused them. Skip them: the real failure follows.
                    if ctx
                        .hook_ctx
                        .fail_fast_aborted
                        .load(std::sync::atomic::Ordering::SeqCst)
                        && crate::error::is_cancellation(&err)
                    {
                        debug!("ignoring cancellation after fail-fast abort: {err:#}");
                        continue;
                    }
                    if crate::error::is_cancellation(&err) && cancelled_by_user(&ctx.hook_ctx) {
                        // Prefer the error that says which command was cancelled
                        // over a bare one from a step that never started.
                        if cancelled
                            .as_ref()
                            .is_none_or(|c| c.chain().count() < err.chain().count())
                        {
                            cancelled = Some(err);
                        }
                        continue;
                    }
                    if ctx.fail_fast {
                        abort_running_steps(&ctx.hook_ctx).await;
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
        if result.is_ok()
            && let Some(err) = cancelled
        {
            result = Err(err);
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

    pub(crate) fn files_in_contention_for(
        &self,
        files: &[PathBuf],
        run_type: RunType,
    ) -> Result<HashSet<PathBuf>> {
        if run_type != RunType::Fix || !self.steps.values().any(|j| j.check_first()) {
            return Ok(Default::default());
        }
        let step_map: HashMap<&str, &Step> = self
            .steps
            .values()
            .map(|step| (step.name.as_str(), step))
            .collect();
        let files_by_step: HashMap<&str, Vec<PathBuf>> = self
            .steps
            .values()
            .map(|step| {
                let step_files = step.filter_files(files)?;

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
    use crate::step::{ArgvCommand, Command};

    #[test]
    fn user_cancellation_does_not_count_as_a_fail_fast_abort() {
        let (failed, aborted) = (CancellationToken::new(), AtomicBool::new(false));
        assert!(!user_cancelled(&failed, &aborted));
        failed.cancel(); // Ctrl-C
        assert!(user_cancelled(&failed, &aborted));
        // A failure reported after Ctrl-C does not claim the abort.
        mark_failure_abort(&failed, &aborted);
        assert!(!aborted.load(Ordering::SeqCst));
        assert!(user_cancelled(&failed, &aborted));
    }

    #[test]
    fn user_cancel_decides_the_outcome_whatever_the_jobs_reported() {
        let allowed = |e: &eyre::Report| e.to_string() == "allowed";
        let token = |cancelled| {
            let t = CancellationToken::new();
            if cancelled {
                t.cancel();
            }
            t
        };
        let none = AtomicBool::new(false);
        // Not cancelled: allowed failures and real ones pass through.
        assert_eq!(
            settle(&token(false), &none, Ok(1), allowed).unwrap(),
            Some(1)
        );
        let err = settle(
            &token(false),
            &none,
            Err::<i32, _>(eyre::eyre!("allowed")),
            allowed,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "allowed");
        // Ctrl-C: Ok and an allowed failure are both a cancelled step.
        let err = settle(&token(true), &none, Ok(1), allowed).unwrap_err();
        assert!(crate::error::is_cancellation(&err));
        let err = settle(
            &token(true),
            &none,
            Err::<i32, _>(eyre::eyre!("allowed")),
            allowed,
        )
        .unwrap_err();
        assert!(crate::error::is_cancellation(&err));
        // Ctrl-C after a real failure: the failure is surfaced.
        let err = settle(
            &token(true),
            &none,
            Err::<i32, _>(eyre::eyre!("real")),
            allowed,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "real");
        // Fail-fast abort: Ok ends quietly, errors are unchanged.
        let aborted = AtomicBool::new(true);
        assert_eq!(
            settle(&token(true), &aborted, Ok(1), allowed).unwrap(),
            None
        );
        let err = settle(
            &token(true),
            &aborted,
            Err::<i32, _>(eyre::eyre!("allowed")),
            allowed,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "allowed");
    }

    #[test]
    fn step_failure_abort_is_recorded_before_cancelling() {
        let (failed, aborted) = (CancellationToken::new(), AtomicBool::new(false));
        mark_failure_abort(&failed, &aborted);
        failed.cancel();
        assert!(aborted.load(Ordering::SeqCst));
        assert!(!user_cancelled(&failed, &aborted));
        // A second failure racing in keeps the flag.
        mark_failure_abort(&failed, &aborted);
        assert!(aborted.load(Ordering::SeqCst));
    }

    #[test]
    fn init_inherits_group_fields_without_merging_child_overrides() {
        let group_shell: Script = "bash -o errexit -c".parse().unwrap();
        let child_shell: Script = "zsh -o errexit -c".parse().unwrap();
        let group_exclude = Pattern::Globs(vec!["**/*.snap".to_string()]);
        let child_exclude = Pattern::Globs(vec!["**/*.fixture.js".to_string()]);

        let inherited_step = Step {
            check: Some("echo inherited".parse().unwrap()),
            exclude: Some(Pattern::Globs(vec![])),
            ..Default::default()
        };

        let override_step = Step {
            check: Some("echo override".parse().unwrap()),
            dir: Some("different/path".to_string()),
            prefix: Some(CommandPrefix::Shell("npm exec --".to_string())),
            workspace_indicator: Some("eslint.config.js".to_string()),
            shell: Some(child_shell.clone()),
            stage: Some(vec!["eslint-output/**".to_string()]),
            exclude: Some(child_exclude.clone()),
            ..Default::default()
        };

        let mut group = StepGroup {
            dir: Some("packages/frontend".to_string()),
            prefix: Some(CommandPrefix::Shell("mise x --".to_string())),
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
        assert_eq!(
            prettier.prefix.as_ref(),
            Some(&CommandPrefix::Shell("mise x --".to_string()))
        );
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
        assert_eq!(
            eslint.prefix.as_ref(),
            Some(&CommandPrefix::Shell("npm exec --".to_string()))
        );
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

    #[test]
    fn init_inherits_argv_prefix_for_structured_command() {
        let step = Step {
            check: Some(Command::Argv(ArgvCommand {
                argv: vec!["ruff".to_string(), "check".to_string()],
            })),
            ..Default::default()
        };
        let prefix =
            CommandPrefix::Argv(vec!["mise".to_string(), "x".to_string(), "--".to_string()]);
        let mut group = StepGroup {
            prefix: Some(prefix.clone()),
            steps: IndexMap::from([("ruff".to_string(), step)]),
            ..Default::default()
        };

        group.init("python").unwrap();

        assert_eq!(group.steps["ruff"].prefix.as_ref(), Some(&prefix));
    }
}
