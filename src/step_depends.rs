use std::collections::HashMap;

use crate::Result;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StepDependencyStatus {
    Pending,
    Done,
    Failed,
}

impl StepDependencyStatus {
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }

    pub fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }
}

pub struct StepDepends {
    depends: HashMap<
        String,
        (
            watch::Sender<StepDependencyStatus>,
            watch::Receiver<StepDependencyStatus>,
        ),
    >,
}

impl StepDepends {
    pub fn new(names: &[&str]) -> Self {
        StepDepends {
            depends: names
                .iter()
                .map(|name| {
                    (
                        name.to_string(),
                        watch::channel(StepDependencyStatus::Pending),
                    )
                })
                .collect(),
        }
    }

    pub fn is_done(&self, step: &str) -> bool {
        self.status(step).is_terminal()
    }

    pub fn status(&self, step: &str) -> StepDependencyStatus {
        let Some((_tx, rx)) = self.depends.get(step) else {
            return StepDependencyStatus::Done;
        };
        *rx.clone().borrow_and_update()
    }

    pub async fn wait_for(&self, step: &str) -> Result<StepDependencyStatus> {
        let Some((_tx, rx)) = self.depends.get(step) else {
            return Ok(StepDependencyStatus::Done);
        };
        let mut rx = rx.clone();
        loop {
            let status = *rx.borrow_and_update();
            if status.is_terminal() {
                return Ok(status);
            }
            rx.changed().await?;
        }
    }

    pub fn mark_done(&self, step: &str) -> Result<()> {
        self.mark(step, StepDependencyStatus::Done)
    }

    pub fn mark_failed(&self, step: &str) -> Result<()> {
        self.mark(step, StepDependencyStatus::Failed)
    }

    fn mark(&self, step: &str, status: StepDependencyStatus) -> Result<()> {
        let Some((tx, _rx)) = self.depends.get(step) else {
            return Ok(());
        };
        tx.send(status)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{StepDependencyStatus, StepDepends};

    #[tokio::test]
    async fn failed_dependency_is_terminal() {
        let depends = StepDepends::new(&["build"]);

        depends.mark_failed("build").unwrap();

        assert!(depends.is_done("build"));
        assert_eq!(
            depends.wait_for("build").await.unwrap(),
            StepDependencyStatus::Failed
        );
    }
}
