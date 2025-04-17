use std::collections::HashMap;

use crate::Result;
use tokio::sync::watch;

pub struct StepDepends {
    depends: HashMap<String, (watch::Sender<bool>, watch::Receiver<bool>)>,
}

impl StepDepends {
    pub fn new(names: &[&str]) -> Self {
        StepDepends {
            depends: names
                .iter()
                .map(|name| (name.to_string(), watch::channel(false)))
                .collect(),
        }
    }

    pub fn is_done(&self, step: &str) -> bool {
        let (_tx, rx) = self.depends.get(step).expect("step not found");
        *rx.clone().borrow_and_update()
    }

    pub async fn wait_for(&self, step: &str) -> Result<()> {
        let (_tx, rx) = self.depends.get(step).expect("step not found");
        let mut rx = rx.clone();
        while !*rx.borrow_and_update() {
            rx.changed().await?;
        }
        Ok(())
    }

    pub fn mark_done(&self, step: &str) -> Result<()> {
        let (tx, _rx) = self.depends.get(step).unwrap();
        tx.send(true)?;
        Ok(())
    }
}
