use crate::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

pub struct FileRwLocks {
    locks: Mutex<BTreeMap<PathBuf, Arc<RwLock<()>>>>,
}

#[derive(Debug, Default)]
#[allow(unused)]
pub struct Flocks {
    read_locks: Vec<OwnedRwLockReadGuard<()>>,
    write_locks: Vec<OwnedRwLockWriteGuard<()>>,
}

impl FileRwLocks {
    pub fn new(files: impl IntoIterator<Item = PathBuf>) -> Self {
        let locks = files
            .into_iter()
            .map(|f| (f, Arc::new(RwLock::new(()))))
            .collect();
        Self {
            locks: Mutex::new(locks),
        }
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.locks.lock().unwrap().keys().cloned().collect()
    }

    pub fn add_files(&self, files: &[PathBuf]) {
        let mut locks = self.locks.lock().unwrap();
        for file in files {
            locks
                .entry(file.clone())
                .or_insert_with(|| Arc::new(RwLock::new(())));
        }
    }

    fn try_read_locks(&self, files: &BTreeSet<&PathBuf>) -> Result<Flocks> {
        let mut locks = self.locks.lock().unwrap();
        let mut read_locks = Vec::new();
        for file in files {
            let lock = self.get_or_create_lock(&mut locks, file);
            let lock = lock
                .try_read_owned()
                .map_err(|_| eyre::eyre!("failed to get read lock {}", file.display()))?;
            read_locks.push(lock);
        }
        Ok(Flocks {
            read_locks,
            write_locks: vec![],
        })
    }

    /// Take read locks on `files` if no writer holds any of them, without waiting.
    pub fn try_read(&self, files: &[PathBuf]) -> Option<Flocks> {
        self.try_read_locks(&lock_order(files)).ok()
    }

    /// Take write locks on `files` if nothing holds any of them, without waiting.
    pub fn try_write(&self, files: &[PathBuf]) -> Option<Flocks> {
        self.try_write_locks(&lock_order(files)).ok()
    }

    /// Acquire read locks on `files`, waiting for any writers to finish.
    ///
    /// Locks are taken in sorted order (see [`lock_order`]) so callers cannot
    /// deadlock against each other.
    pub async fn read_locks(&self, files: &[PathBuf]) -> Flocks {
        let files = lock_order(files);
        match self.try_read_locks(&files) {
            Ok(flocks) => return flocks,
            Err(e) => {
                debug!("failed to get read locks: {e:?}");
            }
        }
        let mut read_locks = Vec::new();
        for file in &files {
            let lock = self.get_or_create_lock(&mut self.locks.lock().unwrap(), file);
            read_locks.push(lock.read_owned().await);
        }
        Flocks {
            read_locks,
            write_locks: vec![],
        }
    }

    fn try_write_locks(&self, files: &BTreeSet<&PathBuf>) -> Result<Flocks> {
        let mut locks = self.locks.lock().unwrap();
        let mut write_locks = Vec::new();
        for file in files {
            let lock = self.get_or_create_lock(&mut locks, file);
            let lock = lock
                .try_write_owned()
                .map_err(|_| eyre::eyre!("failed to get write lock {}", file.display()))?;
            write_locks.push(lock);
        }
        Ok(Flocks {
            read_locks: vec![],
            write_locks,
        })
    }

    /// Acquire write locks on `files`, waiting for any readers or writers to
    /// finish. Locks are taken in sorted order (see [`lock_order`]).
    pub async fn write_locks(&self, files: &[PathBuf]) -> Flocks {
        let files = lock_order(files);
        match self.try_write_locks(&files) {
            Ok(flocks) => return flocks,
            Err(e) => {
                debug!("failed to get write locks: {e:?}");
            }
        }
        let mut write_locks = Vec::new();
        for file in &files {
            let lock = self.get_or_create_lock(&mut self.locks.lock().unwrap(), file);
            write_locks.push(lock.write_owned().await);
        }
        Flocks {
            read_locks: vec![],
            write_locks,
        }
    }

    fn get_or_create_lock(
        &self,
        locks: &mut BTreeMap<PathBuf, Arc<RwLock<()>>>,
        file: &PathBuf,
    ) -> Arc<RwLock<()>> {
        if let Some(lock) = locks.get(file) {
            lock
        } else {
            locks
                .entry(file.clone())
                .or_insert_with(|| Arc::new(RwLock::new(())))
        }
        .clone()
    }
}

/// Sort and deduplicate `files` so every caller acquires locks in the same
/// order. Waiting on locks one at a time in inconsistent orders can deadlock,
/// and locking the same path twice for writing would deadlock on itself.
fn lock_order(files: &[PathBuf]) -> BTreeSet<&PathBuf> {
    files.iter().collect()
}
