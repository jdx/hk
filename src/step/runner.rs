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
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Returns a [`CmdLineRunner`] configured with the platform default shell.
///
/// - **Windows**: execute `cmd.exe /d /s /c <run>` directly so hk adds only a
///   single wrapper.
/// - **Unix**: `sh -o errexit -c <run>`
pub(crate) fn default_shell_cmd(run: &str) -> CmdLineRunner {
    if cfg!(windows) {
        CmdLineRunner::direct("cmd.exe")
            .arg("/d")
            .arg("/s")
            .arg("/c")
            .raw_arg(run)
    } else {
        CmdLineRunner::new("sh")
            .arg("-o")
            .arg("errexit")
            .arg("-c")
            .arg(run)
    }
}

#[cfg(not(windows))]
fn parse_shell(shell: &str) -> Result<Vec<String>> {
    shell_words::split(shell).wrap_err("failed to parse shell command")
}

#[cfg(windows)]
fn parse_shell(shell: &str) -> Result<Vec<String>> {
    split_windows_command_line(shell).wrap_err("failed to parse shell command")
}

#[cfg(windows)]
fn split_windows_command_line(input: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
                while matches!(chars.peek(), Some(' ' | '\t')) {
                    chars.next();
                }
            }
            '\\' => {
                let mut slash_count = 1;
                while matches!(chars.peek(), Some('\\')) {
                    chars.next();
                    slash_count += 1;
                }
                if matches!(chars.peek(), Some('"')) {
                    current.extend(std::iter::repeat_n('\\', slash_count / 2));
                    if slash_count % 2 == 0 {
                        chars.next();
                        in_quotes = !in_quotes;
                    } else {
                        chars.next();
                        current.push('"');
                    }
                } else {
                    current.extend(std::iter::repeat_n('\\', slash_count));
                }
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        eyre::bail!("unterminated quote in shell command");
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

fn shell_program(shell: &str) -> Option<String> {
    parse_shell(shell)
        .ok()
        .and_then(|parts| parts.into_iter().next())
}

pub(crate) fn configured_shell_cmd(
    shell: &str,
    shell_type: ShellType,
    run: &str,
) -> Result<CmdLineRunner> {
    let shell = parse_shell(shell)?;
    if cfg!(windows) && matches!(shell_type, ShellType::Cmd) {
        let mut cmd = CmdLineRunner::direct(shell.first().map(|s| s.as_str()).unwrap_or("cmd.exe"));
        for arg in shell.iter().skip(1) {
            cmd = cmd.arg(arg);
        }
        if !shell
            .iter()
            .skip(1)
            .any(|arg| arg.eq_ignore_ascii_case("/c") || arg.eq_ignore_ascii_case("/k"))
        {
            cmd = cmd.arg("/d").arg("/s").arg("/c");
        }
        return Ok(cmd.raw_arg(run));
    }
    let mut cmd = CmdLineRunner::new(shell.first().map(|s| s.as_str()).unwrap_or("sh"));
    for arg in shell.iter().skip(1) {
        cmd = cmd.arg(arg);
    }
    Ok(cmd.arg(run))
}

fn is_path_var(key: &str) -> bool {
    if cfg!(windows) {
        key.eq_ignore_ascii_case("PATH")
    } else {
        key == "PATH"
    }
}

fn invokes_hk(run: &str) -> bool {
    let tokens = tokenize_command_line(run);
    for segment in tokens.split(|token| is_control_operator(token)) {
        if segment_invokes_hk(segment) {
            return true;
        }
    }
    false
}

fn segment_invokes_hk(segment: &[String]) -> bool {
    let mut i = 0;
    while i < segment.len() {
        let token = &segment[i];
        if is_shell_assignment(token) {
            i += 1;
            continue;
        }
        if is_env_program(token) {
            i += 1;
            while i < segment.len() {
                let token = &segment[i];
                if token == "--" {
                    i += 1;
                    break;
                }
                if token.contains('=') {
                    i += 1;
                    continue;
                }
                if token.starts_with('-') {
                    let takes_value = env_option_takes_value(token);
                    i += 1;
                    if takes_value && !token.contains('=') && i < segment.len() {
                        i += 1;
                    }
                    continue;
                }
                break;
            }
            continue;
        }
        if is_wrapper_program(token) {
            i += 1;
            while i < segment.len() && segment[i].starts_with('-') {
                i += 1;
            }
            continue;
        }
        if is_xargs_program(token) {
            i += 1;
            while i < segment.len() {
                let token = &segment[i];
                if token == "--" {
                    i += 1;
                    break;
                }
                if !token.starts_with('-') {
                    break;
                }
                let takes_value = matches!(
                    token.as_str(),
                    "-E" | "-I" | "-L" | "-P" | "-d" | "-n" | "-s" | "--eof" | "--replace"
                );
                i += 1;
                if takes_value && !token.contains('=') && i < segment.len() {
                    i += 1;
                }
            }
            return i < segment.len() && is_hk_program(&segment[i]);
        }
        return is_hk_program(token);
    }
    false
}

fn is_control_operator(token: &str) -> bool {
    matches!(token, "|" | "||" | "&" | "&&" | ";")
}

fn tokenize_command_line(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut quote = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '\n' || ch == '\r' || ch == ';' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                tokens.push(";".to_string());
            }
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            None if ch == '|' || ch == '&' => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
                if matches!(chars.peek(), Some(next) if *next == ch) {
                    chars.next();
                    tokens.push(format!("{ch}{ch}"));
                } else {
                    tokens.push(ch.to_string());
                }
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn is_hk_program(token: &str) -> bool {
    Path::new(token)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("hk") || name.eq_ignore_ascii_case("hk.exe"))
        .unwrap_or(false)
}

fn is_env_program(token: &str) -> bool {
    Path::new(token)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("env"))
        .unwrap_or(false)
}

fn is_wrapper_program(token: &str) -> bool {
    matches!(token, "command" | "builtin" | "exec" | "nohup")
}

fn is_xargs_program(token: &str) -> bool {
    Path::new(token)
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("xargs"))
        .unwrap_or(false)
}

fn env_option_takes_value(token: &str) -> bool {
    matches!(
        token,
        "-S" | "--split-string" | "-u" | "--unset" | "-C" | "--chdir" | "-a" | "--argv0"
    )
}

fn is_shell_assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        && !name.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn prepend_current_exe_dir_to_path(path: Option<&str>) -> Option<OsString> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let mut paths = vec![exe_dir];
    if let Some(path) = path
        .map(OsString::from)
        .or_else(|| std::env::var_os("PATH"))
    {
        paths.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(paths).ok()
}

pub(crate) fn hk_path_env_for_command(
    run: &str,
    envs: &[(String, String)],
) -> Option<(String, OsString)> {
    if !invokes_hk(run) {
        return None;
    }
    let key = envs
        .iter()
        .rev()
        .find(|(key, _)| is_path_var(key))
        .map(|(key, _)| key.clone())
        .unwrap_or_else(|| "PATH".to_string());
    let path = envs
        .iter()
        .rev()
        .find(|(key, _)| is_path_var(key))
        .map(|(_, value)| value.as_str());
    prepend_current_exe_dir_to_path(path).map(|value| (key, value))
}

pub(crate) fn apply_command_envs(
    mut cmd: CmdLineRunner,
    run: &str,
    envs: &[(String, String)],
) -> CmdLineRunner {
    for (key, value) in envs {
        cmd = cmd.env(key, value);
    }
    if let Some((key, path)) = hk_path_env_for_command(run, envs) {
        cmd = cmd.env(key, path);
    }
    cmd
}

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
        let Some(mut run) = run_cmd
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
        else {
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
            configured_shell_cmd(&shell.to_string(), self.shell_type(), &run)?
        } else {
            default_shell_cmd(&run)
        };
        cmd = cmd
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
        let rendered_env = self
            .env
            .iter()
            .map(|(key, value)| Ok((key.clone(), tera::render(value, &tctx)?)))
            .collect::<Result<Vec<_>>>()?;
        cmd = apply_command_envs(cmd, &run, &rendered_env);
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
        self.run_cmd(run_type)
            .map(|cmd| !cmd.to_string().trim().is_empty())
            .unwrap_or(false)
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
        let shell = shell_program(&shell).unwrap_or(shell);
        let shell = Path::new(&shell)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        // Use case-insensitive matching for shell names
        // Include .exe variants for Windows environments (Git Bash, MSYS2, Cygwin)
        let shell_lower = shell.to_lowercase();
        match shell_lower.as_str() {
            "bash" | "bash.exe" => ShellType::Bash,
            "dash" | "dash.exe" => ShellType::Dash,
            "fish" | "fish.exe" => ShellType::Fish,
            "sh" | "sh.exe" => ShellType::Sh,
            "zsh" | "zsh.exe" => ShellType::Zsh,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_command_for_empty_command() {
        // Mirror the nix_fmt.pkl pattern: windows is empty, other has the real command.
        // On Windows the empty string should make has_command_for return false;
        // on every other platform the `other` fallback provides a valid command.
        let step = Step {
            name: "test_step".to_string(),
            check: Some(Script {
                linux: None,
                macos: None,
                windows: Some("".to_string()),
                other: Some("other_cmd".to_string()),
            }),
            fix: None,
            ..Default::default()
        };

        #[cfg(target_os = "windows")]
        {
            assert!(!step.has_command_for(RunType::Check));
        }

        #[cfg(not(target_os = "windows"))]
        {
            assert!(step.has_command_for(RunType::Check));
        }
    }

    #[test]
    fn test_has_command_for_valid_command() {
        // Test that has_command_for returns true when command is valid
        let step = Step {
            name: "test_step".to_string(),
            check: Some(Script {
                linux: Some("cmd".to_string()),
                macos: Some("cmd".to_string()),
                windows: Some("cmd".to_string()),
                other: Some("cmd".to_string()),
            }),
            fix: None,
            ..Default::default()
        };

        // Should have a command on all platforms
        assert!(step.has_command_for(RunType::Check));
    }

    #[test]
    fn test_has_command_for_no_command() {
        // Test that has_command_for returns false when no command is defined
        let step = Step {
            name: "test_step".to_string(),
            check: None,
            fix: None,
            ..Default::default()
        };

        assert!(!step.has_command_for(RunType::Check));
        assert!(!step.has_command_for(RunType::Fix));
    }

    #[test]
    fn test_default_shell_cmd_constructs_without_panic() {
        let _runner = default_shell_cmd("echo hello");
    }

    #[test]
    fn test_prepend_current_exe_dir_to_path_prepends_current_binary() {
        let path = prepend_current_exe_dir_to_path(Some("/tmp")).unwrap();
        let mut parts = std::env::split_paths(&path);
        assert_eq!(
            parts.next().unwrap(),
            std::env::current_exe().unwrap().parent().unwrap()
        );
    }

    #[test]
    fn test_invokes_hk_detects_nested_hk_calls() {
        assert!(invokes_hk("hk util trailing-whitespace file.txt"));
        assert!(invokes_hk("env FOO=1 hk util trailing-whitespace file.txt"));
        assert!(invokes_hk(
            "/usr/bin/env hk util trailing-whitespace file.txt"
        ));
        assert!(invokes_hk("command hk util trailing-whitespace file.txt"));
        assert!(invokes_hk("FOO=1 hk util trailing-whitespace file.txt"));
        assert!(invokes_hk("xargs hk util trailing-whitespace --fix"));
        assert!(invokes_hk("C:/tools/hk.exe util trailing-whitespace --fix"));
        assert!(invokes_hk("echo ok & hk util trailing-whitespace file.txt"));
        assert!(invokes_hk("echo ok;hk util trailing-whitespace file.txt"));
        assert!(!invokes_hk("typos --diff file.txt"));
        assert!(!invokes_hk("echo hk util trailing-whitespace"));
    }

    #[test]
    fn test_tokenize_command_line_splits_control_operators() {
        assert_eq!(
            tokenize_command_line("echo ok;hk util trailing-whitespace"),
            vec!["echo", "ok", ";", "hk", "util", "trailing-whitespace"]
        );
        assert_eq!(
            tokenize_command_line("echo ok & hk util trailing-whitespace"),
            vec!["echo", "ok", "&", "hk", "util", "trailing-whitespace"]
        );
        assert_eq!(
            tokenize_command_line("echo ok\nhk util trailing-whitespace"),
            vec!["echo", "ok", ";", "hk", "util", "trailing-whitespace"]
        );
    }

    #[test]
    fn test_shell_type_parses_quoted_shell_paths() {
        let step = Step {
            shell: Some(
                r#""C:/Program Files/PowerShell/7/pwsh.exe" -NoLogo -Command"#
                    .parse()
                    .unwrap(),
            ),
            ..Default::default()
        };

        assert!(matches!(step.shell_type(), ShellType::PowerShell));
    }

    #[test]
    #[cfg(windows)]
    fn test_parse_shell_preserves_windows_backslashes() {
        let parsed = parse_shell(r#"C:\Windows\System32\cmd.exe /d /s /c"#).unwrap();
        assert_eq!(parsed[0], r#"C:\Windows\System32\cmd.exe"#);
    }

    #[test]
    #[cfg(windows)]
    fn test_default_shell_cmd_handles_quoted_windows_path() {
        use std::fs;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("quoted path.txt");
        fs::write(&path, "hello from windows\n").unwrap();

        let run = format!(r#"type "{}""#, path.display());
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(default_shell_cmd(&run).execute()).unwrap();

        assert!(result.stdout.contains("hello from windows"));
    }

    #[test]
    #[cfg(windows)]
    fn test_configured_cmd_shell_keeps_single_wrap() {
        let runner = configured_shell_cmd("cmd.exe", ShellType::Cmd, "echo hello").unwrap();
        assert_eq!(format!("{runner}"), "cmd.exe /d /s /c echo hello");
    }
}
