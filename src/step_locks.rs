use crate::file_rw_locks::Flocks;
use tokio::sync::OwnedSemaphorePermit;

#[allow(unused)]
#[derive(Debug)]
pub struct StepLocks {
    flocks: Flocks,
    semaphore: OwnedSemaphorePermit,
}

impl StepLocks {
    pub fn new(flocks: Flocks, semaphore: OwnedSemaphorePermit) -> Self {
        Self { flocks, semaphore }
    }

    /// Release the file locks, keeping the job slot.
    pub fn into_semaphore(self) -> OwnedSemaphorePermit {
        self.semaphore
    }
}
