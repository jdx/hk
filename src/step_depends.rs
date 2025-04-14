use crate::Result;
use crate::step_job::StepJob;
use indexmap::IndexMap;
use itertools::Itertools;
use tokio::sync::watch;

pub struct StepDepends {
    depends: IndexMap<String, (watch::Sender<usize>, watch::Receiver<usize>)>,
    remaining_counts: IndexMap<String, std::sync::Mutex<usize>>,
}

impl StepDepends {
    pub fn new(jobs: &[StepJob]) -> Self {
        let names = jobs.iter().map(|s| s.step.name.clone()).collect::<Vec<_>>();
        let counts = names.iter().counts();
        StepDepends {
            depends: names
                .iter()
                .map(|name| (name.to_string(), watch::channel(counts[name])))
                .collect(),
            remaining_counts: names
                .iter()
                .map(|name| (name.to_string(), std::sync::Mutex::new(counts[name])))
                .collect(),
        }
    }

    pub fn is_done(&self, step: &str) -> bool {
        if let Some(remaining) = self.remaining_counts.get(step) {
            *remaining.lock().unwrap() == 0
        } else {
            true
        }
    }

    pub async fn wait_for(&self, step: &str) -> Result<()> {
        let (_tx, rx) = self.depends.get(step).expect("step not found");
        let mut rx = rx.clone();
        while *rx.borrow_and_update() > 0 {
            rx.changed().await?;
        }
        Ok(())
    }

    pub fn job_done(&self, step: &str) {
        let remaining = self.remaining_counts.get(step).unwrap();
        *remaining.lock().unwrap() -= 1;
    }
}
