use crate::config::Steps;
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
    step::{LinterStep, RunType},
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
    steps: Vec<Arc<Steps>>,
    files: Vec<PathBuf>,
    run_type: RunType,
}

impl StepQueueBuilder {
    pub fn new(steps: Vec<Arc<Steps>>, files: Vec<PathBuf>, run_type: RunType) -> Self {
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
                match step.as_ref() {
                    Steps::Run(s) => {
                        if s.exclusive || groups.is_empty() {
                            groups.push(vec![]);
                        }
                        groups.last_mut().unwrap().push(step);
                        if s.exclusive {
                            groups.push(vec![]);
                        }
                    }
                    Steps::Linter(s) => {
                        if s.exclusive || groups.is_empty() {
                            groups.push(vec![]);
                        }
                        groups.last_mut().unwrap().push(step);
                        if s.exclusive {
                            groups.push(vec![]);
                        }
                    }
                    Steps::Stash(_stash) => {
                        groups.push(vec![]);
                        groups.last_mut().unwrap().push(step);
                    }
                }
                groups
            })
            .into_iter()
            .filter(|group| !group.is_empty())
            .map(|group| self.build_queue_for_group(&group))
            .collect::<Result<_>>()?;

        Ok(StepQueue { groups })
    }

    fn build_queue_for_group(&self, steps: &[&Arc<Steps>]) -> Result<Vec<StepJob>> {
        let jobs = LazyCell::new(|| Settings::get().jobs().get());
        let mut queue = vec![];
        for step in steps {
            // TODO: remove this clone
            let step = match step.as_ref() {
                Steps::Run(s) => Arc::new(Steps::Run((*s).clone())),
                Steps::Linter(s) => Arc::new(Steps::Linter((*s).clone())),
                Steps::Stash(s) => Arc::new(Steps::Stash((*s).clone())),
            };
            let Some(run_type) = step.available_run_type(self.run_type) else {
                debug!("{step}: skipping step due to no available run type");
                continue;
            };
            // Check if step should be skipped based on HK_SKIP_STEPS
            if crate::env::HK_SKIP_STEPS.contains(step.name()) {
                debug!("{step}: skipping step due to HK_SKIP_STEPS");
                continue;
            }
            if !step.is_profile_enabled() {
                debug!("{step}: skipping step due to profile not being enabled");
                continue;
            }
            let mut files = self.files.clone();
            if let Some(dir) = step.dir() {
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
            if let Some(glob) = step.glob() {
                files = glob::get_matches(glob, &files)?;
                if files.is_empty() {
                    debug!("{step}: no matches for step");
                    continue;
                }
            }
            let jobs = match &*step {
                Steps::Linter(s) => {
                    if let Some(workspace_indicators) = s.workspaces_for_files(&files)? {
                        let job = StepJob::new(step.clone(), files.clone(), run_type);
                        workspace_indicators
                            .into_iter()
                            .map(|workspace_indicator| {
                                job.clone().with_workspace_indicator(workspace_indicator)
                            })
                            .collect()
                    } else if s.batch {
                        files
                            .chunks((files.len() / *jobs).max(1))
                            .map(|chunk| StepJob::new(step.clone(), chunk.to_vec(), run_type))
                            .collect()
                    } else {
                        vec![StepJob::new(step, files.clone(), run_type)]
                    }
                }
                _ => vec![StepJob::new(step, files.clone(), run_type)],
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
        steps: &[&Arc<Steps>],
        files: &[PathBuf],
    ) -> Result<HashSet<PathBuf>> {
        let steps = steps
            .iter()
            .filter_map(|s| match s.as_ref() {
                Steps::Run(_) => unimplemented!("run steps are not supported in step queue"),
                Steps::Linter(s) => Some(s),
                Steps::Stash(_) => None,
            })
            .collect::<Vec<_>>();
        let step_map: HashMap<String, &LinterStep> = steps
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
        let mut steps_per_file: HashMap<&Path, Vec<&LinterStep>> = Default::default();
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
