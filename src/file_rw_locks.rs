use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

type FileLock = Arc<RwLock<()>>;

pub struct FileRwLocks {
    inner: Mutex<Inner>,
}

struct Inner {
    /// One lock per path. A path's lock is never replaced, nor removed once
    /// steps run, so a handle looked up once stays valid for the rest of the
    /// hook run.
    ///
    /// A hash map rather than a `BTreeMap`: comparing `Path`s walks their
    /// components, so an ordered lookup costs about ten times as much, and a
    /// step over every file in the repository looks up thousands of paths.
    locks: HashMap<PathBuf, FileLock>,
    /// Every path in `locks`, in sorted order.
    files: Vec<PathBuf>,
    /// Whether paths were pushed onto `files` since it was last sorted.
    files_unsorted: bool,
}

#[derive(Debug, Default)]
#[allow(unused)]
pub struct Flocks {
    read_locks: Vec<OwnedRwLockReadGuard<()>>,
    write_locks: Vec<OwnedRwLockWriteGuard<()>>,
}

impl FileRwLocks {
    pub fn new(files: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut inner = Inner {
            locks: HashMap::new(),
            files: Vec::new(),
            files_unsorted: false,
        };
        for file in files {
            inner.get_or_create_lock(&file);
        }
        Self {
            inner: Mutex::new(inner),
        }
    }

    /// Every path known to this run, sorted.
    pub fn files(&self) -> Vec<PathBuf> {
        let mut inner = self.inner.lock().unwrap();
        if inner.files_unsorted {
            inner.files.sort_unstable();
            inner.files_unsorted = false;
        }
        inner.files.clone()
    }

    pub fn add_files(&self, files: &[PathBuf]) {
        let mut inner = self.inner.lock().unwrap();
        for file in files {
            inner.get_or_create_lock(file);
        }
    }

    /// Forgets `files`. Only for use before any step runs: afterwards a step
    /// may hold a handle to one of their locks.
    pub fn remove_files(&self, files: &[PathBuf]) {
        let mut inner = self.inner.lock().unwrap();
        let mut removed = false;
        for file in files {
            removed |= inner.locks.remove(file).is_some();
        }
        if removed {
            let Inner { locks, files, .. } = &mut *inner;
            files.retain(|f| locks.contains_key(f));
        }
    }

    /// Take read locks on `files` if no writer holds any of them, without waiting.
    pub fn try_read(&self, files: &[PathBuf]) -> Option<Flocks> {
        self.try_lock(files, try_read_all).1
    }

    /// Take write locks on `files` if nothing holds any of them, without waiting.
    pub fn try_write(&self, files: &[PathBuf]) -> Option<Flocks> {
        self.try_lock(files, try_write_all).1
    }

    /// Acquire read locks on `files`, waiting for any writers to finish.
    ///
    /// Locks are taken in sorted order (see [`lock_order`]) so callers cannot
    /// deadlock against each other.
    pub async fn read_locks(&self, files: &[PathBuf]) -> Flocks {
        let locks = match self.try_lock(files, try_read_all) {
            (_, Some(flocks)) => return flocks,
            (locks, None) => locks,
        };
        debug!("failed to get read locks, waiting");
        let mut read_locks = Vec::with_capacity(locks.len());
        for lock in locks {
            read_locks.push(lock.read_owned().await);
        }
        Flocks {
            read_locks,
            write_locks: vec![],
        }
    }

    /// Acquire write locks on `files`, waiting for any readers or writers to
    /// finish. Locks are taken in sorted order (see [`lock_order`]).
    pub async fn write_locks(&self, files: &[PathBuf]) -> Flocks {
        let locks = match self.try_lock(files, try_write_all) {
            (_, Some(flocks)) => return flocks,
            (locks, None) => locks,
        };
        debug!("failed to get write locks, waiting");
        let mut write_locks = Vec::with_capacity(locks.len());
        for lock in locks {
            write_locks.push(lock.write_owned().await);
        }
        Flocks {
            read_locks: vec![],
            write_locks,
        }
    }

    /// Look up the locks for `files` in lock order and try to take them all
    /// without waiting. The lookups and the attempt happen under one hold of
    /// the map mutex, so two callers' attempts never interleave. Returns the
    /// locks in order either way, so a caller that has to wait for them does
    /// not need to look them up again.
    fn try_lock(
        &self,
        files: &[PathBuf],
        try_all: fn(&[FileLock]) -> Option<Flocks>,
    ) -> (Vec<FileLock>, Option<Flocks>) {
        let files = lock_order(files);
        let mut inner = self.inner.lock().unwrap();
        let locks = files
            .into_iter()
            .map(|file| inner.get_or_create_lock(file))
            .collect::<Vec<_>>();
        let flocks = try_all(&locks);
        (locks, flocks)
    }
}

impl Inner {
    fn get_or_create_lock(&mut self, file: &PathBuf) -> FileLock {
        if let Some(lock) = self.locks.get(file) {
            return lock.clone();
        }
        let lock = FileLock::default();
        self.locks.insert(file.clone(), lock.clone());
        if !self.files_unsorted && self.files.last().is_some_and(|last| last >= file) {
            self.files_unsorted = true;
        }
        self.files.push(file.clone());
        lock
    }
}

fn try_read_all(locks: &[FileLock]) -> Option<Flocks> {
    let read_locks = locks
        .iter()
        .map(|lock| lock.clone().try_read_owned().ok())
        .collect::<Option<Vec<_>>>()?;
    Some(Flocks {
        read_locks,
        write_locks: vec![],
    })
}

fn try_write_all(locks: &[FileLock]) -> Option<Flocks> {
    let write_locks = locks
        .iter()
        .map(|lock| lock.clone().try_write_owned().ok())
        .collect::<Option<Vec<_>>>()?;
    Some(Flocks {
        read_locks: vec![],
        write_locks,
    })
}

/// Sort and deduplicate `files` so every caller acquires locks in the same
/// order. Waiting on locks one at a time in inconsistent orders can deadlock,
/// and locking the same path twice for writing would deadlock on itself.
///
/// Job file lists usually come from the sorted hook file list already, so
/// this only sorts when they are not strictly increasing.
fn lock_order(files: &[PathBuf]) -> Vec<&PathBuf> {
    let mut files = files.iter().collect::<Vec<_>>();
    if !files.is_sorted_by(|a, b| a < b) {
        files.sort_unstable();
        files.dedup();
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn files_are_sorted_after_additions() {
        let locks = FileRwLocks::new(paths(&["b", "d"]));
        locks.add_files(&paths(&["c", "a", "d"]));
        assert_eq!(locks.files(), paths(&["a", "b", "c", "d"]));
        locks.add_files(&paths(&["e"]));
        assert_eq!(locks.files(), paths(&["a", "b", "c", "d", "e"]));
    }

    #[test]
    fn removed_files_can_be_added_again() {
        let locks = FileRwLocks::new(paths(&["b", "a", "c"]));
        locks.remove_files(&paths(&["b", "missing"]));
        assert_eq!(locks.files(), paths(&["a", "c"]));
        locks.add_files(&paths(&["b"]));
        assert_eq!(locks.files(), paths(&["a", "b", "c"]));
    }

    #[test]
    fn lock_order_sorts_and_deduplicates() {
        let files = paths(&["b", "a/c", "b", "a"]);
        assert_eq!(
            lock_order(&files),
            paths(&["a", "a/c", "b"]).iter().collect::<Vec<_>>()
        );
        // Paths that are equal component by component are one lock.
        let files = paths(&["a//b", "a/b"]);
        assert_eq!(lock_order(&files).len(), 1);
    }

    #[test]
    fn writer_excludes_readers_and_writers() {
        let locks = FileRwLocks::new(paths(&["a", "b"]));
        let write = locks.try_write(&paths(&["b"])).unwrap();
        assert!(locks.try_read(&paths(&["a", "b"])).is_none());
        assert!(locks.try_write(&paths(&["b"])).is_none());
        // A failed attempt releases what it took, so `a` is still free.
        assert!(locks.try_write(&paths(&["a"])).is_some());
        drop(write);
        let read = locks.try_read(&paths(&["a", "b"])).unwrap();
        assert!(locks.try_read(&paths(&["b"])).is_some());
        assert!(locks.try_write(&paths(&["b"])).is_none());
        drop(read);
        assert!(locks.try_write(&paths(&["a", "b", "new"])).is_some());
        assert_eq!(locks.files(), paths(&["a", "b", "new"]));
    }

    #[tokio::test]
    async fn waiting_writer_gets_locks_when_released() {
        let locks = Arc::new(FileRwLocks::new(paths(&["a", "b"])));
        let read = locks.try_read(&paths(&["b"])).unwrap();
        let waiter = tokio::spawn({
            let locks = locks.clone();
            async move {
                let _flocks = locks.write_locks(&paths(&["b", "a"])).await;
            }
        });
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        drop(read);
        waiter.await.unwrap();
        assert!(locks.try_write(&paths(&["a", "b"])).is_some());
    }
}
