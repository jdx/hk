//! Output capture and fix suggestions.
//!
//! This module handles:
//! - Saving command output for the end-of-run summary
//! - Generating helpful "to fix, run:" suggestions when checks fail

use crate::step_context::StepContext;
use crate::step_job::StepJob;
use crate::tera;
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
    /// * `cmd_result` - Optional command result (for filtering files from check_list_files output)
    pub(crate) fn collect_fix_suggestion(
        &self,
        ctx: &StepContext,
        job: &StepJob,
        cmd_result: Option<&ensembler::CmdResult>,
    ) {
        // Only suggest fixes when the entire hook run is in check mode,
        // not when an individual job temporarily runs a check (e.g., check_first during a fix run)
        if !matches!(ctx.hook_ctx.run_type, RunType::Check) || self.fix.is_none() {
            return;
        }
        // Prefer filtering files if check_list_files output is available
        let mut suggest_files = job.files.clone();
        if let Some(result) = cmd_result
            && self.check_list_files.is_some()
        {
            let (files, _extras) = self.filter_files_from_check_list(&job.files, &result.stdout);
            if !files.is_empty() {
                suggest_files = files;
            }
        }
        // Build a minimal context based on the suggested files, honoring dir/workspace
        let temp_job = StepJob::new(Arc::new(self.clone()), suggest_files, RunType::Fix);
        let suggest_ctx = temp_job.tctx(&ctx.hook_ctx.tctx);
        if let Some(mut fix_cmd) = self.run_cmd(RunType::Fix).map(|s| s.to_string()) {
            if let Some(prefix) = &self.prefix {
                fix_cmd = format!("{prefix} {fix_cmd}");
            }
            if let Ok(rendered) = tera::render(&fix_cmd, &suggest_ctx) {
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
}
