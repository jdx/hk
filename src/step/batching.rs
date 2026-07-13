//! Job batching to prevent ARG_MAX overflow.
//!
//! When passing large file lists to shell commands, the total argument length can exceed
//! the operating system's `ARG_MAX` limit, causing "Argument list too long" errors.
//!
//! This module renders the actual run command for each job and only splits jobs whose
//! rendered command exceeds the safe limit. Steps whose commands don't reference
//! `{{files}}` (or render to a small string for any other reason) are left as a
//! single job, even when the underlying file list is large.

use crate::env;
use crate::step_job::StepJob;
use crate::tera;
use eyre::{Result, bail};
use std::path::PathBuf;
use std::sync::Arc;

use super::{
    ShellType,
    types::{Command, Step},
};

const CMD_COMMAND_LINE_LIMIT: usize = 8191;
const CMD_COMMAND_LINE_SAFE_LIMIT: usize = CMD_COMMAND_LINE_LIMIT / 2;
const WINDOWS_CREATE_PROCESS_LIMIT: usize = 32767;
const WINDOWS_CREATE_PROCESS_SAFE_LIMIT: usize = WINDOWS_CREATE_PROCESS_LIMIT / 2;
#[cfg(target_os = "linux")]
// Linux limits each argv/envp string to 32 pages, independently of ARG_MAX.
const LINUX_MAX_ARG_STRLEN_PAGES: usize = 32;

#[cfg(target_os = "linux")]
fn platform_max_arg_strlen() -> Option<usize> {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size > 0 {
        Some((page_size as usize).saturating_mul(LINUX_MAX_ARG_STRLEN_PAGES))
    } else {
        Some(128 * 1024)
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_max_arg_strlen() -> Option<usize> {
    None
}

fn shell_command_safe_limit(arg_max: usize, max_arg_strlen: Option<usize>) -> usize {
    let aggregate_limit = arg_max / 2;
    match max_arg_strlen {
        // execve includes the terminating NUL in MAX_ARG_STRLEN.
        Some(max_arg_strlen) => aggregate_limit.min(max_arg_strlen.saturating_sub(1)),
        None => aggregate_limit,
    }
}

impl Step {
    fn auto_batch_safe_limit(&self, command: &Command) -> usize {
        if command.is_argv() {
            return if cfg!(windows) {
                WINDOWS_CREATE_PROCESS_SAFE_LIMIT
            } else {
                *env::ARG_MAX / 2
            };
        }
        match self.shell_type() {
            ShellType::Cmd => CMD_COMMAND_LINE_SAFE_LIMIT,
            _ => shell_command_safe_limit(*env::ARG_MAX, platform_max_arg_strlen()),
        }
    }

    /// Estimates the size of the `{{files}}` template variable expansion.
    ///
    /// Used as a fallback when rendering the run command fails (e.g. the
    /// template references a variable not yet present in the context).
    pub(crate) fn estimate_files_string_size(&self, files: &[PathBuf]) -> usize {
        files
            .iter()
            .map(|f| {
                let path_str = f.to_str().unwrap_or("");
                // Worst-case quoted size: 2x + 2 (quotes), +1 for space separator
                path_str.len() * 2 + 2 + 1
            })
            .sum()
    }

    /// Render the run command for a hypothetical job containing `files` and return
    /// its byte length. Returns `None` if rendering fails (e.g. the template
    /// references a variable not in the context).
    fn render_run_command_size(
        &self,
        original_job: &StepJob,
        files: &[PathBuf],
        base_tctx: &tera::Context,
    ) -> Option<usize> {
        let run_cmd = if original_job.check_first {
            self.check_first_cmd().map(|cmd| cmd.command())
        } else {
            self.run_cmd(original_job.run_type)
        }?;
        if run_cmd.is_empty() {
            return None;
        }

        let mut temp = StepJob::new(
            Arc::clone(&original_job.step),
            files.to_vec(),
            original_job.run_type,
        );
        temp.check_first = original_job.check_first;
        if let Some(wi) = original_job.workspace_indicator() {
            temp = temp.with_workspace_indicator(wi.clone());
        }
        let tctx = temp.tctx(base_tctx);
        run_cmd
            .render(&tctx, self.prefix.as_deref())
            .ok()
            .map(|command| command.execution_size())
    }

    /// Automatically batch jobs whose rendered run command would exceed the safe exec limit.
    ///
    /// Uses 50% of ARG_MAX as an aggregate safety margin and also respects
    /// platform-specific per-command limits such as Linux's MAX_ARG_STRLEN.
    /// Renders the actual run command with each candidate file subset; if the rendered command
    /// fits, no batching is performed. Otherwise binary-searches the largest batch size whose
    /// rendered command still fits.
    ///
    /// If rendering fails for any reason, falls back to estimating the size of the quoted
    /// file-list expansion — preserves the previous (purely size-based) behavior as a safety net.
    pub(crate) fn auto_batch_jobs(
        &self,
        jobs: Vec<StepJob>,
        base_tctx: &tera::Context,
    ) -> Result<Vec<StepJob>> {
        self.auto_batch_jobs_with_limit(jobs, base_tctx, None)
    }

    fn auto_batch_jobs_with_limit(
        &self,
        jobs: Vec<StepJob>,
        base_tctx: &tera::Context,
        safe_limit_override: Option<usize>,
    ) -> Result<Vec<StepJob>> {
        if self.stdin.is_some() {
            // stdin path doesn't pass files via argv; never auto-batch
            return Ok(jobs);
        }

        let mut batched_jobs = Vec::with_capacity(jobs.len());

        for job in jobs {
            if job.skip_reason.is_some() || job.files.is_empty() {
                batched_jobs.push(job);
                continue;
            }

            let run_cmd = if job.check_first {
                self.check_first_cmd().map(|cmd| cmd.command())
            } else {
                self.run_cmd(job.run_type)
            };
            let safe_limit = safe_limit_override.unwrap_or_else(|| {
                run_cmd
                    .map(|command| self.auto_batch_safe_limit(command))
                    .unwrap_or(*env::ARG_MAX / 2)
            });

            // Try render-based sizing first; fall back to byte estimation on render failure.
            let full_size = self
                .render_run_command_size(&job, &job.files, base_tctx)
                .unwrap_or_else(|| self.estimate_files_string_size(&job.files));

            if full_size <= safe_limit {
                batched_jobs.push(job);
                continue;
            }

            debug!(
                "{}: auto-batching {} files (rendered size: {} bytes, limit: {} bytes)",
                self.name,
                job.files.len(),
                full_size,
                safe_limit
            );

            // Size every chunk independently: later paths may be much longer
            // than the paths at the start of the job.
            let mut offset = 0;
            while offset < job.files.len() {
                let remaining = &job.files[offset..];
                let single_size = self
                    .render_run_command_size(&job, &remaining[..1], base_tctx)
                    .unwrap_or_else(|| self.estimate_files_string_size(&remaining[..1]));
                if single_size > safe_limit {
                    bail!(
                        "{}: rendered command for {} is {} bytes, exceeding the {}-byte command-line limit",
                        self.name,
                        remaining[0].display(),
                        single_size,
                        safe_limit
                    );
                }

                // Binary search the largest prefix of the remaining files that fits.
                let mut low = 1;
                let mut high = remaining.len();
                while low < high {
                    let mid = (low + high).div_ceil(2);
                    let test_size = self
                        .render_run_command_size(&job, &remaining[..mid], base_tctx)
                        .unwrap_or_else(|| self.estimate_files_string_size(&remaining[..mid]));
                    if test_size <= safe_limit {
                        low = mid;
                    } else {
                        high = mid - 1;
                    }
                }
                let batch_size = low;

                debug!("{}: using batch size of {} files", self.name, batch_size);

                let chunk = &remaining[..batch_size];
                let mut new_job = StepJob::new(Arc::clone(&job.step), chunk.to_vec(), job.run_type);
                // Preserve job-level state that isn't reconstructed by StepJob::new.
                new_job.check_first = job.check_first;
                new_job.skip_reason = job.skip_reason.clone();
                if let Some(wi) = job.workspace_indicator() {
                    new_job = new_job.with_workspace_indicator(wi.clone());
                }
                batched_jobs.push(new_job);
                offset += batch_size;
            }
        }

        Ok(batched_jobs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::RunType;
    use crate::step_job::StepJob;

    #[test]
    fn cmd_shell_uses_cmd_command_line_limit() {
        let step = Step {
            shell: Some("cmd.exe".parse().unwrap()),
            check: Some("echo {{files}}".parse().unwrap()),
            ..Default::default()
        };

        assert_eq!(
            step.auto_batch_safe_limit(step.check.as_ref().unwrap()),
            CMD_COMMAND_LINE_SAFE_LIMIT
        );
    }

    #[test]
    fn structured_argv_uses_aggregate_process_limit() {
        let command = Command::Argv(super::super::types::ArgvCommand {
            argv: vec!["echo".to_string(), "{{files}}".to_string()],
        });
        let step = Step::default();
        let expected = if cfg!(windows) {
            WINDOWS_CREATE_PROCESS_SAFE_LIMIT
        } else {
            *env::ARG_MAX / 2
        };

        assert_eq!(step.auto_batch_safe_limit(&command), expected);
    }

    #[test]
    fn non_cmd_shell_uses_arg_max_limit() {
        let step = Step {
            shell: Some("sh".parse().unwrap()),
            ..Default::default()
        };

        let expected = shell_command_safe_limit(*env::ARG_MAX, platform_max_arg_strlen());
        let command: Command = "echo {{files}}".parse().unwrap();
        assert_eq!(step.auto_batch_safe_limit(&command), expected);
    }

    #[test]
    fn linux_shell_limit_accounts_for_max_arg_strlen() {
        assert_eq!(
            shell_command_safe_limit(2 * 1024 * 1024, Some(128 * 1024)),
            128 * 1024 - 1
        );
    }

    #[test]
    fn cmd_shell_auto_batches_below_unix_arg_max() {
        let step = Step {
            name: "test".to_string(),
            shell: Some("cmd.exe".parse().unwrap()),
            check: Some("echo {{files}}".parse().unwrap()),
            ..Default::default()
        };
        let files = (0..400)
            .map(|i| {
                PathBuf::from(format!(
                    "directory/with/a/long/path/file_with_a_long_name_{i}.txt"
                ))
            })
            .collect();
        let job = StepJob::new(Arc::new(step.clone()), files, RunType::Check);

        let jobs = step
            .auto_batch_jobs(vec![job], &tera::Context::default())
            .unwrap();

        assert!(jobs.len() > 1);
    }

    #[test]
    fn auto_batch_sizes_each_chunk_independently() {
        let step = Step {
            name: "test".to_string(),
            check: Some("echo {{files}}".parse().unwrap()),
            ..Default::default()
        };
        let mut files = (0..20)
            .map(|i| PathBuf::from(format!("s{i}")))
            .collect::<Vec<_>>();
        files.extend(
            (0..20)
                .map(|i| PathBuf::from(format!("long-directory-name-{i:02}/long-file-name.txt"))),
        );
        let job = StepJob::new(Arc::new(step.clone()), files, RunType::Check);
        let tctx = tera::Context::default();

        let jobs = step
            .auto_batch_jobs_with_limit(vec![job], &tctx, Some(100))
            .unwrap();

        assert!(jobs.len() > 1);
        assert!(jobs.iter().all(|job| {
            step.render_run_command_size(job, &job.files, &tctx)
                .unwrap()
                <= 100
        }));
    }

    #[test]
    fn auto_batch_rejects_oversized_single_file_command() {
        let step = Step {
            name: "test".to_string(),
            check: Some(
                format!("echo {} {{{{files}}}}", "x".repeat(100))
                    .parse()
                    .unwrap(),
            ),
            ..Default::default()
        };
        let job = StepJob::new(
            Arc::new(step.clone()),
            vec![PathBuf::from("file.txt")],
            RunType::Check,
        );

        let err = step
            .auto_batch_jobs_with_limit(vec![job], &tera::Context::default(), Some(100))
            .unwrap_err();

        assert!(err.to_string().contains("file.txt"));
        assert!(err.to_string().contains("100-byte command-line limit"));
    }
}

impl Step {
    /// Check if this step has any file filters configured.
    ///
    /// Used to determine if an empty file list means "no matching files"
    /// versus "run on all files".
    pub(crate) fn has_filters(&self) -> bool {
        self.glob.is_some()
            || self.match_any.is_some()
            || self.dir.is_some()
            || self
                .exclude
                .as_ref()
                .is_some_and(|pattern| !pattern.is_empty())
            || self.types.is_some()
    }
}
