use crate::error::XXResult;
use std::env;
use std::path::{Path, PathBuf};

use crate::file::display_path;
use crate::hash::hash_to_str;
use crate::{XXError, file};

pub type OnLockedFn = Box<dyn Fn(&Path)>;

pub struct FSLock {
    path: PathBuf,
    on_locked: Option<OnLockedFn>,
}

impl FSLock {
    pub fn new(path: &Path) -> Self {
        Self {
            path: env::temp_dir().join("fslock").join(hash_to_str(&path)),
            on_locked: None,
        }
    }

    pub fn with_callback<F>(mut self, cb: F) -> Self
    where
        F: Fn(&Path) + 'static,
    {
        self.on_locked = Some(Box::new(cb));
        self
    }

    pub fn lock(self) -> XXResult<fslock::LockFile> {
        if let Some(parent) = self.path.parent() {
            file::mkdirp(parent)?;
        }
        let mut lock = fslock::LockFile::open(&self.path)
            .map_err(|e| XXError::FSLockError(e, format!("lockfile {}", self.path.display())))?;
        if !lock
            .try_lock()
            .map_err(|e| XXError::FSLockError(e, format!("lockfile {}", self.path.display())))?
        {
            if let Some(f) = self.on_locked {
                f(&self.path)
            }
            lock.lock().map_err(|e| {
                XXError::FSLockError(e, format!("lockfile {}", self.path.display()))
            })?;
        }
        Ok(lock)
    }
}

pub fn get(path: &Path, force: bool) -> XXResult<Option<fslock::LockFile>> {
    let lock = if force {
        None
    } else {
        let lock = FSLock::new(path)
            .with_callback(|l| {
                debug!("waiting for lock on {}", display_path(l));
            })
            .lock()?;
        Some(lock)
    };
    Ok(lock)
}
