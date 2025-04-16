use crate::step_job::StepJob;
use crate::{step::LinterStep, step_context::StepContext};

use crate::{Result, settings::Settings};
use std::{
    cell::LazyCell,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{glob, step::RunType};

/// Takes a list of steps and files as input and builds a queue of jobs that would need to be
/// executed by StepScheduler
///
/// This is kept outside of the Scheduler so the logic here is pure where the scheduler deals with
/// parallel execution synchronization.
pub struct StepQueue {
    queue: VecDeque<(Arc<LinterStep>, Option<VecDeque<StepJob>>)>,
}

fn files_in_contention(
    run_type: RunType,
    steps: &[&Arc<LinterStep>],
    files: &[PathBuf],
) -> Result<HashSet<PathBuf>> {
    let step_map: HashMap<&str, &LinterStep> = steps
        .iter()
        .map(|step| (step.name.as_str(), &***step))
        .collect();
    let files_by_step: HashMap<&str, Vec<PathBuf>> = steps
        .iter()
        .map(|step| {
            let files = glob::get_matches(step.glob.as_ref().unwrap_or(&vec![]), files)?;
            Ok((step.name.as_str(), files))
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
            .any(|step| step.available_run_type(run_type) == Some(RunType::Fix))
        {
            files_in_contention.insert(file.to_path_buf());
        }
    }

    Ok(files_in_contention)
}

impl StepQueue {
    pub(crate) fn new(group: &[Arc<LinterStep>]) -> Self {
        Self { queue: group.iter().map(|step| (step.clone(), None)).collect() }
    }

    pub(crate) fn next_job(&mut self, ctx: &StepContext) -> Result<Option<StepJob>> {
        for _ in 0..self.queue.len() {
            if let Some((step, jobs)) = self.queue.pop_front() {
                let mut jobs = match jobs {
                    Some(jobs) => jobs,
                    None => {
                        if !step.can_run(ctx) {
                            self.queue.push_back((step, None));
                            continue;
                        }
                        step.build_jobs(ctx.files(), ctx.run_type)?
                    }
                };
                if let Some(job) = jobs.pop_front() {
                    self.queue.push_back((step, Some(jobs)));
                    return Ok(Some(job));
                }
            }
        }
        // TODO
        // if q.iter().any(|j| j.check_first) {
        //     let files_in_contention = self.files_in_contention(steps, &self.files)?;
        //     for job in q.iter_mut().filter(|j| j.check_first) {
        //         // only set check_first if there are any files in contention
        //         job.check_first = job.files.iter().any(|f| files_in_contention.contains(f));
        //     }
        // }
        Ok(None)
    }

    pub(crate) fn is_done(&self) -> bool {
        self.queue.is_empty()
    }

    pub(crate) fn group_steps(steps: &[Arc<LinterStep>]) -> Vec<Vec<Arc<LinterStep>>> {
        steps
            .iter()
            .fold(vec![], |mut groups, step| {
                if step.exclusive || groups.is_empty() {
                    groups.push(vec![]);
                }
                groups.last_mut().unwrap().push(step.clone());
                if step.exclusive {
                    groups.push(vec![]);
                }
                groups
            })
    }
}
