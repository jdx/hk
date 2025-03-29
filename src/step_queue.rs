use crate::step_job::StepJob;

use crate::{Result, settings::Settings};
use std::{
    cell::LazyCell,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    glob,
    step::{RunType, Step},
};

/// Takes a list of steps and files as input and builds a queue of jobs that would need to be
/// executed by StepScheduler
///
/// This is kept outside of the Scheduler so the logic here is pure where the scheduler deals with
/// parallel execution synchronization.
pub struct StepQueue {
    pub groups: Vec<Vec<StepJob>>,
}

pub struct StepQueueBuilder {
    steps: Vec<Arc<Step>>,
    files: Vec<PathBuf>,
    run_type: RunType,
}

impl StepQueueBuilder {
    pub fn new(steps: Vec<Arc<Step>>, files: Vec<PathBuf>, run_type: RunType) -> Self {
        Self {
            steps,
            files,
            run_type,
        }
    }

    pub fn build(&self) -> Result<StepQueue> {
        // groups is a list of list of steps which are separated by exclusive steps
        // any exclusive step will be in a group by itself
        let groups = self
            .steps
            .iter()
            .fold(vec![], |mut groups, step| {
                if step.exclusive || groups.is_empty() {
                    groups.push(vec![]);
                }
                groups.last_mut().unwrap().push(step);
                groups
            })
            .into_iter()
            .map(|group| self.build_queue_for_group(&group))
            .collect::<Result<_>>()?;

        Ok(StepQueue { groups })
    }

    fn build_queue_for_group(&self, steps: &[&Arc<Step>]) -> Result<Vec<StepJob>> {
        let jobs = LazyCell::new(|| Settings::get().jobs().get());
        let mut queue = vec![];
        for step in steps {
            let Some(run_type) = step.available_run_type(self.run_type) else {
                debug!("{step}: skipping step due to no available run type");
                continue;
            };
            if !step.is_profile_enabled() {
                debug!("{step}: skipping step due to profile not being enabled");
                continue;
            }
            let mut files = self.files.clone();
            if let Some(dir) = &step.dir {
                files.retain(|f| f.starts_with(dir));
                if files.is_empty() {
                    debug!("{step}: no matches for step in {dir}");
                    continue;
                }
                for f in files.iter_mut() {
                    // strip the dir prefix from the file path
                    *f = f.strip_prefix(dir).unwrap_or(f).to_path_buf();
                }
            }
            if let Some(glob) = &step.glob {
                files = glob::get_matches(glob, &files)?;
                if files.is_empty() {
                    debug!("{step}: no matches for step");
                    continue;
                }
            }
            let step = (*step).clone();
            let jobs = if let Some(workspace_indicators) = step.workspaces_for_files(&files)? {
                let job = StepJob::new(step.clone(), files.clone(), run_type);
                workspace_indicators
                    .into_iter()
                    .map(|workspace_indicator| {
                        job.clone().with_workspace_indicator(workspace_indicator)
                    })
                    .collect()
            } else if step.batch {
                files
                    .chunks((files.len() / *jobs).max(1))
                    .map(|chunk| StepJob::new(step.clone(), chunk.to_vec(), run_type))
                    .collect()
            } else {
                vec![StepJob::new(step, files.clone(), run_type)]
            };
            queue.push(jobs);
        }
        let mut q = vec![];
        // round robin through the steps to avoid just 1 step running
        while !queue.is_empty() {
            for jobs in queue.iter_mut() {
                if let Some(job) = jobs.pop() {
                    q.push(job);
                }
            }
            queue.retain(|jobs| !jobs.is_empty());
        }

        if q.iter().any(|j| j.check_first) {
            let files_in_contention = self.files_in_contention(steps, &self.files)?;
            for job in q.iter_mut().filter(|j| j.check_first) {
                // only set check_first if there are any files in contention
                job.check_first = job.files.iter().any(|f| files_in_contention.contains(f));
            }
        }
        Ok(q)
    }

    fn files_in_contention(
        &self,
        steps: &[&Arc<Step>],
        files: &[PathBuf],
    ) -> Result<HashSet<PathBuf>> {
        let step_map: HashMap<String, &Step> = steps
            .iter()
            .map(|step| (step.name.clone(), step.as_ref()))
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
}
