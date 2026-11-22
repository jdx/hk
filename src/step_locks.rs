use crate::file_rw_locks::Flocks;
use crate::passed_checks::WriteWatch;
use tokio::sync::{OwnedRwLockReadGuard, OwnedSemaphorePermit};

#[allow(unused)]
#[derive(Debug)]
pub struct StepLocks {
    /// Declared first so it is dropped, comparing the files it watches, before
    /// the file locks are released.
    watch: Option<WriteWatch>,
    flocks: Flocks,
    semaphore: OwnedSemaphorePermit,
    command_guard: Option<OwnedRwLockReadGuard<()>>,
}

impl StepLocks {
    pub fn new(
        watch: Option<WriteWatch>,
        flocks: Flocks,
        semaphore: OwnedSemaphorePermit,
        command_guard: OwnedRwLockReadGuard<()>,
    ) -> Self {
        Self {
            watch,
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

    /// Release the file locks, keeping the job slot.
    pub fn into_semaphore(self) -> OwnedSemaphorePermit {
        self.semaphore
    }
}
