//! Single step job execution.
//!
//! This module contains the `run` method that executes a single job.
//! It handles:
//!
//! - Condition evaluation
//! - Profile checking
//! - Template rendering
//! - Command building and execution
//! - Output capture
//! - Error handling and progress updates

use crate::hook::SkipReason;
use crate::step_context::StepContext;
use crate::step_job::{StepJob, StepJobStatus};
use crate::timings::StepTimingGuard;
use crate::{Result, tera};
use clx::progress::ProgressStatus;
use ensembler::CmdLineRunner;
use eyre::WrapErr;
use itertools::Itertools;
use std::path::PathBuf;
use std::process::Stdio;

use super::expr_env::EXPR_ENV;
use super::shell::ShellType;
use super::types::{Pattern, RunType, Script, Step};
use crate::error::Error;

impl Step {
    /// Execute a single job.
    ///
    /// This is the core execution function that runs a command for a step.
    /// It handles the full lifecycle from condition checking through command
    /// execution and result handling.
    ///
    /// # Execution Flow
    ///
    /// 1. Check if hook has already failed (abort early)
    /// 2. Evaluate condition expression (if configured)
    /// 3. Check profile requirements
    /// 4. Filter out deleted files
    /// 5. Acquire semaphore and start job
    /// 6. Render command template
    /// 7. Execute command
    /// 8. Handle success/failure
    /// 9. Update progress
    ///
    /// # Arguments
    ///
    /// * `ctx` - The step execution context
    /// * `job` - The job to execute (modified in place)
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err` if the command fails
    pub(crate) async fn run(&self, ctx: &StepContext, job: &mut StepJob) -> Result<()> {
        if ctx.hook_ctx.failed.is_cancelled() {
            trace!("{self}: skipping step due to previous failure");
            // Hide the job progress if it was created
            if let Some(progress) = &job.progress {
                progress.set_status(ProgressStatus::Hide);
            }
            return Ok(());
        }
        if let Some(job_condition) = &self.job_condition {
            let val = EXPR_ENV.eval(job_condition, &ctx.hook_ctx.expr_ctx())?;
            debug!("{self}: condition: {job_condition} = {val}");
            if val == expr::Value::Bool(false) {
                self.mark_skipped(ctx, &SkipReason::ConditionFalse)?;
                return Ok(());
            }
        }
        // After evaluating the condition, check profiles so condition-false wins over profiles
        if let Some(reason) = self.profile_skip_reason() {
            self.mark_skipped(ctx, &reason)?;
            return Ok(());
        }
        job.progress = Some(job.build_progress(ctx));
        job.status = StepJobStatus::Pending;
        let semaphore = if let Some(semaphore) = job.semaphore.take() {
            semaphore
        } else {
            ctx.hook_ctx.semaphore().await
        };
        job.status_start(ctx, semaphore).await?;
        // Filter out files that no longer exist (e.g., deleted by parallel tasks)
        // Use symlink_metadata to check if the path exists as a file/symlink (even if broken)
        job.files.retain(|f| f.symlink_metadata().is_ok());
        // Skip this job if all files were deleted
        if job.files.is_empty() && self.has_filters() {
            debug!("{self}: all files deleted before execution");
            self.mark_skipped(ctx, &SkipReason::NoFilesToProcess)?;
            return Ok(());
        }
        let mut tctx = job.tctx(&ctx.hook_ctx.tctx);
        // Set {{globs}} template variable based on pattern type
        match self.glob.as_ref() {
            Some(Pattern::Globs(g)) => {
                tctx.with_globs(g.as_slice());
            }
            Some(Pattern::Regex { pattern, .. }) => {
                // For regex patterns, provide the pattern string so templates can use it
                tctx.insert("globs", pattern);
            }
            None => {
                tctx.with_globs(&[] as &[&str]);
            }
        }
        let file_msg = |files: &[PathBuf]| {
            format!(
                "{} file{}",
                files.len(),
                if files.len() == 1 { "" } else { "s" }
            )
        };
        let run_cmd = if job.check_first {
            self.check_first_cmd()
        } else {
            self.run_cmd(job.run_type)
        };
        let Some(mut run) = run_cmd.map(|s| s.to_string()) else {
            eyre::bail!("{self}: no run command");
        };
        if let Some(prefix) = &self.prefix {
            run = format!("{prefix} {run}");
        }
        let run = tera::render(&run, &tctx)
            .wrap_err_with(|| format!("{self}: failed to render command template"))?;
        let pattern_display = match &self.glob {
            Some(Pattern::Globs(g)) => g.join(" "),
            Some(Pattern::Regex { pattern, .. }) => format!("regex: {}", pattern),
            None => String::new(),
        };
        job.progress.as_ref().unwrap().prop(
            "message",
            &format!("{} – {} – {}", file_msg(&job.files), pattern_display, run),
        );
        job.progress.as_ref().unwrap().update();
        if log::log_enabled!(log::Level::Trace) {
            for file in &job.files {
                trace!("{self}: {}", file.display());
            }
        }
        let mut cmd = if let Some(shell) = &self.shell {
            let shell = shell.to_string();
            let shell = shell.split_whitespace().collect_vec();
            let mut cmd = CmdLineRunner::new(shell[0]);
            for arg in shell[1..].iter() {
                cmd = cmd.arg(arg);
            }
            cmd
        } else if cfg!(windows) {
            CmdLineRunner::new("cmd.exe").arg("/c")
        } else {
            CmdLineRunner::new("sh").arg("-o").arg("errexit").arg("-c")
        };
        cmd = cmd
            .arg(&run)
            .with_pr(job.progress.as_ref().unwrap().clone())
            .with_cancel_token(ctx.hook_ctx.failed.clone())
            .show_stderr_on_error(false)
            .stderr_to_progress(true);
        if let Some(stdin) = &self.stdin {
            let rendered_stdin = tera::render(stdin, &tctx)?;
            cmd = cmd.stdin_string(rendered_stdin);
        }

        if self.interactive {
            clx::progress::pause();
            cmd = cmd
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
        }
        if let Some(dir) = &self.dir {
            cmd = cmd.current_dir(dir);
        }
        for (key, value) in &self.env {
            let value = tera::render(value, &tctx)?;
            cmd = cmd.env(key, value);
        }
        let timing_guard = StepTimingGuard::new(ctx.hook_ctx.timing.clone(), self);
        let exec_result = cmd.execute().await;
        timing_guard.finish();
        if self.interactive {
            clx::progress::resume();
        }
        match exec_result {
            Ok(result) => {
                // For both check_list_files and check_diff: stderr is informational only
                // Files are read from stdout; stderr may contain warnings, debug info, etc.
                if run_cmd == self.check_list_files.as_ref() {
                    debug!(
                        "{self}: check_list_files succeeded (exit 0), stdout len={}, stderr len={}",
                        result.stdout.len(),
                        result.stderr.len()
                    );
                    if !result.stderr.trim().is_empty() {
                        debug!("{self}: check_list_files stderr output:\n{}", result.stderr);
                    }
                    // Warn if exit 0 but stdout has content (misconfigured tool)
                    if !result.stdout.trim().is_empty() {
                        warn!(
                            "{self}: check_list_files exited 0 (success) but returned files in stdout. This may indicate misconfiguration - the tool should exit non-zero when files need fixing."
                        );
                    }
                } else if run_cmd == self.check_diff.as_ref() {
                    // For check_diff, stderr with exit 0 is just informational (e.g., "N files already formatted")
                    debug!(
                        "{self}: check_diff succeeded (exit 0), stdout len={}, stderr len={}",
                        result.stdout.len(),
                        result.stderr.len()
                    );
                }
                // Save output for end-of-run summary based on configured mode
                self.save_output_summary(
                    ctx,
                    job,
                    &result.stdout,
                    &result.stderr,
                    &result.combined_output,
                    false, // not a failure
                );
            }
            Err(err) => {
                if let ensembler::Error::ScriptFailed(e) = &err {
                    if job.check_first
                        && (run_cmd == self.check_list_files.as_ref()
                            || run_cmd == self.check_diff.as_ref())
                    {
                        return Err(Error::CheckListFailed {
                            source: eyre::eyre!("{}", err),
                            stdout: e.3.stdout.clone(),
                            stderr: e.3.stderr.clone(),
                        })?;
                    }
                    // Save output from a failed command as well
                    self.save_output_summary(
                        ctx,
                        job,
                        &e.3.stdout,
                        &e.3.stderr,
                        &e.3.combined_output,
                        true, // is a failure
                    );

                    // If we're in check mode and a fix command exists, collect a helpful suggestion
                    self.collect_fix_suggestion(ctx, job, Some(&e.3));
                }
                if job.check_first && job.run_type == RunType::Check {
                    ctx.progress.set_status(ProgressStatus::Warn);
                } else {
                    ctx.progress.set_status(ProgressStatus::Failed);
                }
                return Err(err).wrap_err(run);
            }
        }
        ctx.decrement_job_count();
        job.status_finished()?;
        Ok(())
    }
}

impl Step {
    /// Initialize the step with its name and validate configuration.
    ///
    /// Must be called after deserialization to set the step name and
    /// validate that incompatible options aren't set together.
    ///
    /// # Arguments
    ///
    /// * `name` - The step name from the configuration
    ///
    /// # Errors
    ///
    /// Returns an error if both `stdin` and `interactive` are set.
    pub(crate) fn init(&mut self, name: &str) -> Result<()> {
        if self.stdin.is_some() && self.interactive {
            eyre::bail!(
                "Step '{}' can't have both `stdin` and `interactive = true`.",
                name
            );
        }
        self.name = name.to_string();
        if self.interactive {
            self.exclusive = true;
        }
        Ok(())
    }

    /// Get the command to run for the given run type.
    ///
    /// For Fix mode, returns the fix command if available, otherwise falls back to check.
    /// For Check mode, returns check, check_diff, or check_list_files (in that preference order).
    pub fn run_cmd(&self, run_type: RunType) -> Option<&Script> {
        match run_type {
            RunType::Fix => {
                self.fix
                    .as_ref()
                    // NB: Even if we don't have a fix command,
                    // we still can run the `check` command.
                    .or(self.run_cmd(RunType::Check))
            }
            RunType::Check => self
                .check
                .as_ref()
                .or(self.check_diff.as_ref())
                .or(self.check_list_files.as_ref()),
        }
    }

    /// Get the command to run in "check first" mode.
    ///
    /// Prefers check_diff, then check, then check_list_files.
    pub fn check_first_cmd(&self) -> Option<&Script> {
        self.check_diff
            .as_ref()
            .or(self.check.as_ref())
            .or(self.check_list_files.as_ref())
    }

    /// Check if this step has a command for the given run type.
    pub fn has_command_for(&self, run_type: RunType) -> bool {
        self.run_cmd(run_type).is_some()
    }

    /// Get the shell type for this step.
    ///
    /// Parses the shell configuration to determine the shell type,
    /// defaulting to Sh on Unix or Cmd on Windows if not specified.
    pub fn shell_type(&self) -> ShellType {
        let shell = self
            .shell
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let shell = shell.split_whitespace().next().unwrap_or_default();
        let shell = shell.split(['/', '\\']).next_back().unwrap_or_default();
        match shell {
            "bash" | "bash.exe" => ShellType::Bash,
            "dash" => ShellType::Dash,
            "fish" => ShellType::Fish,
            "sh" => ShellType::Sh,
            "zsh" => ShellType::Zsh,
            "cmd" | "cmd.exe" => ShellType::Cmd,
            "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe" => ShellType::PowerShell,
            "" if cfg!(windows) => ShellType::Cmd,
            "" => ShellType::Sh,
            _ => ShellType::Other(shell.to_string()),
        }
    }

    /// Mark this step as skipped with the given reason.
    ///
    /// Updates the progress display and marks dependencies as satisfied.
    pub fn mark_skipped(&self, ctx: &StepContext, reason: &SkipReason) -> Result<()> {
        // Track all skip reasons for potential future use
        ctx.hook_ctx.track_skip(&self.name, reason.clone());

        if reason.should_display() {
            ctx.progress.prop("message", &reason.message());
            let status =
                ProgressStatus::DoneCustom(crate::ui::style::eblue("⇢").bold().to_string());
            ctx.progress.set_status(status);
        } else {
            // Step is skipped but message shouldn't be displayed
            ctx.progress.set_status(ProgressStatus::Hide);
        }
        ctx.depends.mark_done(&self.name)?;
        Ok(())
    }
}
