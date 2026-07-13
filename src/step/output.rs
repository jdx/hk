//! Output capture and fix suggestions.
//!
//! This module handles:
//! - Saving command output for the end-of-run summary
//! - Generating helpful "to fix, run:" suggestions when checks fail

use crate::step_context::StepContext;
use crate::step_job::StepJob;
use crate::ui::style;
use std::sync::Arc;

use super::types::{OutputSummary, RunType, Step};

impl Step {
    /// Save command output for the end-of-run summary.
    ///
    /// Based on the step's `output_summary` setting, captures the appropriate
    /// output (stderr, stdout, combined, or none) to display after all steps complete.
    ///
    /// Skips saving output for check_first checks that failed (since they'll be
    /// followed by a fix that will have its own output).
    ///
    /// # Arguments
    ///
    /// * `ctx` - The step context
    /// * `job` - The current job
    /// * `stdout` - Command stdout
    /// * `stderr` - Command stderr
    /// * `combined` - Interleaved stdout/stderr
    /// * `is_failure` - Whether the command failed
    pub(crate) fn save_output_summary(
        &self,
        ctx: &StepContext,
        job: &StepJob,
        stdout: &str,
        stderr: &str,
        combined: &str,
        is_failure: bool,
    ) {
        // Only skip if this is a check_first check that FAILED (will be followed by a fix)
        // If the check passed, we want to show its output since no fix will run
        let is_check_first_check_that_failed =
            job.check_first && matches!(job.run_type, RunType::Check) && is_failure;
        if is_check_first_check_that_failed {
            return;
        }

        if is_failure {
            ctx.hook_ctx.mark_step_failed(&self.name);
        }

        // On failure, use combined output so diagnostic messages are never
        // lost regardless of which stream the tool writes to — but keep
        // the configured label so tests/users see the expected header.
        // If the step explicitly opted out with `output_summary = "hide"`,
        // respect that even on failure.
        if is_failure && self.output_summary != OutputSummary::Hide {
            ctx.hook_ctx
                .append_step_output(&self.name, self.output_summary.clone(), combined)
        } else {
            match self.output_summary {
                OutputSummary::Stderr => {
                    ctx.hook_ctx
                        .append_step_output(&self.name, OutputSummary::Stderr, stderr)
                }
                OutputSummary::Stdout => {
                    ctx.hook_ctx
                        .append_step_output(&self.name, OutputSummary::Stdout, stdout)
                }
                OutputSummary::Combined => {
                    ctx.hook_ctx
                        .append_step_output(&self.name, OutputSummary::Combined, combined)
                }
                OutputSummary::Hide => {}
            }
        }
    }

    /// Collect a helpful fix suggestion when a check fails.
    ///
    /// When running in check mode and a step fails, this generates a message
    /// like "To fix, run: eslint --fix src/file.ts" to help the user.
    ///
    /// For multi-line commands, suggests `hk fix -S <step>` instead of the
    /// full command.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The step context
    /// * `job` - The current job
    /// * `run_cmd` - Optional command that produced cmd_result
    /// * `cmd_result` - Optional command result (for filtering files from check output)
    pub(crate) fn collect_fix_suggestion(
        &self,
        ctx: &StepContext,
        job: &StepJob,
        run_cmd: Option<&super::types::Command>,
        cmd_result: Option<&ensembler::CmdResult>,
    ) {
        // Only suggest fixes when the entire hook run is in check mode,
        // not when an individual job temporarily runs a check (e.g., check_first during a fix run)
        if !matches!(ctx.hook_ctx.run_type, RunType::Check) || self.fix.is_none() {
            return;
        }
        // Prefer filtering files when the failed check command can identify
        // the files needing fixes.
        let mut suggest_files = job.files.clone();
        if let (Some(run_cmd), Some(result)) = (run_cmd, cmd_result) {
            let (files, parser) = if Some(run_cmd) == self.check_diff.as_ref() {
                let (files, _extras) =
                    self.filter_files_from_check_diff(&job.files, &result.stdout);
                (files, Some("check_diff"))
            } else if Some(run_cmd) == self.check_list_files.as_ref() {
                let (files, _extras) =
                    self.filter_files_from_check_list(&job.files, &result.stdout);
                (files, Some("check_list_files"))
            } else {
                (vec![], None)
            };
            if !files.is_empty() {
                suggest_files = files;
            } else if let Some(parser) = parser {
                debug!(
                    "{self}: {parser} output did not match any job files; suggesting all job files"
                );
            }
        }
        // Build a minimal context based on the suggested files, honoring dir/workspace
        let temp_job = StepJob::new(Arc::new(self.clone()), suggest_files, RunType::Fix);
        let suggest_ctx = temp_job.tctx(&ctx.hook_ctx.tctx);
        if let Some(fix_cmd) = self
            .run_cmd(RunType::Fix)
            .filter(|command| !command.is_empty())
            && let Ok(rendered) = fix_cmd.render(&suggest_ctx, self.prefix.as_deref())
        {
            let rendered = rendered.display(self.shell_type());
            let is_multi_line = rendered.contains('\n');
            if is_multi_line {
                // Too long to inline; suggest hk fix with step filter
                let step_flag = format!("-S {}", &self.name);
                let cmd = format!(
                    "To fix, run: {}",
                    style::edim(format!("hk fix {}", step_flag))
                );
                ctx.hook_ctx.add_fix_suggestion(cmd);
            } else {
                let cmd = format!("To fix, run: {}", style::edim(rendered));
                ctx.hook_ctx.add_fix_suggestion(cmd);
            }
        }
    }
}
