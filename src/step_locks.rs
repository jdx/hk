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
}
