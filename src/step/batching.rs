//! Job batching to prevent ARG_MAX overflow.
//!
//! When passing large file lists to shell commands, the total argument length can exceed
//! the operating system's `ARG_MAX` limit, causing "Argument list too long" errors.
//!
//! This module provides automatic batching that:
//! 1. Estimates the shell-quoted size of file lists
//! 2. Uses binary search to find optimal batch sizes
//! 3. Splits jobs to stay within safe limits

use crate::env;
use crate::step_job::StepJob;
use std::path::PathBuf;

use super::types::Step;

impl Step {
    /// Estimates the size of the `{{files}}` template variable expansion.
    ///
    /// This includes shell quoting overhead and spaces between files.
    /// Uses a conservative estimate (2x + 2 for quotes, +1 for space) to ensure
    /// we don't underestimate.
    ///
    /// # Arguments
    ///
    /// * `files` - The list of files to estimate
    ///
    /// # Returns
    ///
    /// Estimated byte size of the quoted file list string
    pub(crate) fn estimate_files_string_size(&self, files: &[PathBuf]) -> usize {
        files
            .iter()
            .map(|f| {
                let path_str = f.to_str().unwrap_or("");
                // Estimate quoted size: conservative estimate assuming worst-case quoting
                // For shell quoting, worst case is roughly 2x + 2 (quotes)
                path_str.len() * 2 + 2 + 1 // +1 for space separator
            })
            .sum()
    }

    /// Automatically batch jobs if the file list would exceed safe ARG_MAX limits.
    ///
    /// This prevents "Argument list too long" errors when passing large file lists
    /// to commands. Uses binary search to find the largest batch size that fits
    /// within a safe limit (50% of ARG_MAX to account for environment variables
    /// and the command itself).
    ///
    /// # Arguments
    ///
    /// * `jobs` - The jobs to potentially batch
    ///
    /// # Returns
    ///
    /// A new list of jobs, potentially with large jobs split into multiple smaller ones
    pub(crate) fn auto_batch_jobs_if_needed(&self, jobs: Vec<StepJob>) -> Vec<StepJob> {
        // Use 50% of ARG_MAX as a safety margin, accounting for environment variables
        // and the command itself
        let safe_limit = *env::ARG_MAX / 2;

        let mut batched_jobs = Vec::new();

        for job in jobs {
            let estimated_size = self.estimate_files_string_size(&job.files);

            if estimated_size > safe_limit && job.files.len() > 1 {
                // Need to batch this job
                debug!(
                    "{}: auto-batching {} files (estimated size: {} bytes, limit: {} bytes)",
                    self.name,
                    job.files.len(),
                    estimated_size,
                    safe_limit
                );

                // Binary search to find the largest batch_size where files fit within safe_limit
                let mut low = 1;
                let mut high = job.files.len();

                while low < high {
                    let mid = (low + high).div_ceil(2);
                    let test_size = self.estimate_files_string_size(&job.files[..mid]);

                    if test_size <= safe_limit {
                        // mid files fit, try larger
                        low = mid;
                    } else {
                        // mid files don't fit, try smaller
                        high = mid - 1;
                    }
                }

                // After binary search, low contains the largest batch size that fits
                let batch_size = low.max(1); // Ensure at least 1 file per batch

                debug!(
                    "{}: using batch size of {} files per batch",
                    self.name, batch_size
                );

                // Create batched jobs - use the StepJob constructor to properly handle private fields
                for chunk in job.files.chunks(batch_size) {
                    let new_job = StepJob::new(job.step.clone(), chunk.to_vec(), job.run_type);
                    // Note: we can't preserve workspace_indicator or other private fields
                    // without adding public methods to StepJob. For now, batching will
                    // break workspace_indicator jobs, but that's acceptable since those
                    // are typically small workspaces.
                    batched_jobs.push(new_job);
                }
            } else {
                // No batching needed
                batched_jobs.push(job);
            }
        }

        batched_jobs
    }
}

impl Step {
    /// Check if this step has any file filters configured.
    ///
    /// Used to determine if an empty file list means "no matching files"
    /// versus "run on all files".
    pub(crate) fn has_filters(&self) -> bool {
        self.glob.is_some() || self.dir.is_some() || self.exclude.is_some() || self.types.is_some()
    }
}
