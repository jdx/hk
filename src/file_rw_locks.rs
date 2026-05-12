use crate::Result;
use std::{
    collections::BTreeMap,
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

    fn ordered_files(files: &[PathBuf]) -> Vec<PathBuf> {
        let mut files = files.to_vec();
        files.sort();
        files.dedup();
        files
    }

    fn try_read_locks(&self, files: &[PathBuf]) -> Result<Flocks> {
        let files = Self::ordered_files(files);
        let mut locks = self.locks.lock().unwrap();
        let mut read_locks = Vec::new();
        for file in &files {
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

    pub async fn read_locks(&self, files: &[PathBuf]) -> Flocks {
        match self.try_read_locks(files) {
            Ok(flocks) => return flocks,
            Err(e) => {
                debug!("failed to get read locks: {e:?}");
            }
        }
        let files = Self::ordered_files(files);
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

    fn try_write_locks(&self, files: &[PathBuf]) -> Result<Flocks> {
        let files = Self::ordered_files(files);
        let mut locks = self.locks.lock().unwrap();
        let mut write_locks = Vec::new();
        for file in &files {
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

    pub async fn write_locks(&self, files: &[PathBuf]) -> Flocks {
        match self.try_write_locks(files) {
            Ok(flocks) => return flocks,
            Err(e) => {
                debug!("failed to get write locks: {e:?}");
            }
        }
        let files = Self::ordered_files(files);
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

#[cfg(test)]
mod tests {
    use super::FileRwLocks;
    use std::path::PathBuf;

    #[test]
    fn lock_order_is_sorted_and_deduplicated() {
        let files = vec![PathBuf::from("b"), PathBuf::from("a"), PathBuf::from("b")];

        assert_eq!(
            FileRwLocks::ordered_files(&files),
            vec![PathBuf::from("a"), PathBuf::from("b")]
        );
    }
}
