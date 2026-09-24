use crate::file_rw_locks::Flocks;
use tokio::sync::{OwnedRwLockReadGuard, OwnedSemaphorePermit};

#[allow(unused)]
#[derive(Debug)]
pub struct StepLocks {
    flocks: Flocks,
    semaphore: OwnedSemaphorePermit,
    command_guard: Option<OwnedRwLockReadGuard<()>>,
}

impl StepLocks {
    pub fn new(
        flocks: Flocks,
        semaphore: OwnedSemaphorePermit,
        command_guard: OwnedRwLockReadGuard<()>,
    ) -> Self {
        Self {
            flocks,
            semaphore,
            command_guard: Some(command_guard),
        }
    }

    /// Release shared command access before waiting for exclusive diff access,
    /// while retaining the job's input-file locks and semaphore permit.
    pub fn release_command_guard(&mut self) {
        self.command_guard.take();
    }
}
