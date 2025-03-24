use crate::{step_depends::StepDepends, tera};
use indexmap::IndexMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, Semaphore};

/// Stores all the information/mutexes needed to run a StepJob
pub struct StepContext {
    pub file_locks: IndexMap<PathBuf, Arc<RwLock<()>>>,
    pub semaphore: Arc<Semaphore>,
    pub failed: Arc<Mutex<bool>>,
    pub depends: StepDepends,
    pub tctx: tera::Context,
}
