//! Read/write locks on the files of a hook run.
//!
//! A job takes every lock it needs at once or none of them, so a waiting job
//! holds nothing. Taking locks one file at a time instead lets a waiting job
//! sit on the files it already has: a type checker waiting for one fixer would
//! keep a second fixer, which shares no files with the first, waiting for the
//! type checker, and the three would run one after another.
//!
//! A waiting job starts as soon as no job holding locks conflicts with it.
//! Waiting jobs form a queue, which decides only which of them goes first
//! when a release frees files that several want. A waiting job never holds up
//! a later one: a step over every file, such as a whitespace fixer, would
//! otherwise make each later job that shares any file with it wait for every
//! job ahead of it.
//!
//! No job waits while holding locks, so there is no lock order to follow and
//! no deadlock, as long as a caller releases the locks it holds before asking
//! for more. Take every lock a piece of work needs in one call.

use std::{
    collections::{HashMap, VecDeque},
    fmt,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::oneshot;

pub struct FileRwLocks {
    table: Arc<Mutex<Table>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Read,
    Write,
}

struct Table {
    /// Each path's position in `paths` and in the per-file vectors below. A
    /// path keeps its position for the rest of the hook run, unless it is
    /// removed before any step runs.
    ///
    /// A hash map rather than a `BTreeMap`: comparing `Path`s walks their
    /// components, so an ordered lookup costs about ten times as much, and a
    /// step over every file in the repository looks up thousands of paths.
    /// `Path` hashes by component, so `a//b` and `a/b` are one file.
    index: HashMap<PathBuf, usize>,
    paths: Vec<PathBuf>,
    /// The position of every path in `index`, sorted by path when `sorted` is
    /// true.
    order: Vec<usize>,
    sorted: bool,
    /// Jobs holding each file: any number of readers, or one writer.
    readers: Vec<u32>,
    writer: Vec<bool>,
    /// Jobs waiting for locks, oldest first. After every change to the table,
    /// each of them conflicts with a job holding locks.
    queue: VecDeque<Waiter>,
    next_id: u64,
}

struct Waiter {
    id: u64,
    mode: Mode,
    files: Arc<[usize]>,
    ready: oneshot::Sender<()>,
}

/// Locks held on a set of files, released when dropped.
#[derive(Default)]
pub struct Flocks {
    held: Option<Held>,
}

struct Held {
    table: Arc<Mutex<Table>>,
    mode: Mode,
    files: Arc<[usize]>,
}

impl fmt::Debug for Flocks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.held {
            Some(held) => write!(f, "Flocks({:?}, {} files)", held.mode, held.files.len()),
            None => write!(f, "Flocks(none)"),
        }
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        let mut table = self.table.lock().unwrap();
        table.release(&self.files, self.mode);
        table.wake();
    }
}

/// A queued request, from the waiting task's side. If the task is cancelled,
/// dropping this takes the request out of the queue, or releases the locks if
/// they were granted in the meantime.
struct Wait {
    table: Arc<Mutex<Table>>,
    id: u64,
    mode: Mode,
    files: Arc<[usize]>,
    granted: bool,
}

impl Wait {
    fn into_flocks(mut self) -> Flocks {
        self.granted = true;
        Flocks {
            held: Some(Held {
                table: self.table.clone(),
                mode: self.mode,
                files: self.files.clone(),
            }),
        }
    }
}

impl Drop for Wait {
    fn drop(&mut self) {
        if self.granted {
            return;
        }
        let mut table = self.table.lock().unwrap();
        if let Some(pos) = table.queue.iter().position(|w| w.id == self.id) {
            table.queue.remove(pos);
        } else {
            table.release(&self.files, self.mode);
            table.wake();
        }
    }
}

impl FileRwLocks {
    pub fn new(files: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut table = Table {
            index: HashMap::new(),
            paths: Vec::new(),
            order: Vec::new(),
            sorted: true,
            readers: Vec::new(),
            writer: Vec::new(),
            queue: VecDeque::new(),
            next_id: 0,
        };
        for file in files {
            table.position(&file);
        }
        Self {
            table: Arc::new(Mutex::new(table)),
        }
    }

    /// Every path known to this run, sorted.
    pub fn files(&self) -> Vec<PathBuf> {
        let mut table = self.table.lock().unwrap();
        let table = &mut *table;
        if !table.sorted {
            let paths = &table.paths;
            table
                .order
                .sort_unstable_by(|&a, &b| paths[a].cmp(&paths[b]));
            table.sorted = true;
        }
        table
            .order
            .iter()
            .map(|&i| table.paths[i].clone())
            .collect()
    }

    pub fn add_files(&self, files: &[PathBuf]) {
        let mut table = self.table.lock().unwrap();
        for file in files {
            table.position(file);
        }
    }

    /// Forgets `files`. Only for use before any step runs: afterwards a job
    /// may hold or wait for one of their locks.
    pub fn remove_files(&self, files: &[PathBuf]) {
        let mut table = self.table.lock().unwrap();
        let mut removed = false;
        for file in files {
            removed |= table.index.remove(file).is_some();
        }
        if removed {
            // A removed path keeps its unused position; added again, it gets a
            // new one.
            let Table {
                index,
                paths,
                order,
                ..
            } = &mut *table;
            order.retain(|&i| index.get(&paths[i]) == Some(&i));
        }
    }

    /// Take read locks on `files` if no writer holds any of them, without
    /// waiting.
    pub fn try_read(&self, files: &[PathBuf]) -> Option<Flocks> {
        self.try_lock(files, Mode::Read)
    }

    /// Take write locks on `files` if nothing holds any of them, without
    /// waiting.
    pub fn try_write(&self, files: &[PathBuf]) -> Option<Flocks> {
        self.try_lock(files, Mode::Write)
    }

    /// Acquire read locks on `files`, waiting for any writers to finish.
    pub async fn read_locks(&self, files: &[PathBuf]) -> Flocks {
        self.lock(files, Mode::Read).await
    }

    /// Acquire write locks on `files`, waiting for any readers or writers to
    /// finish.
    pub async fn write_locks(&self, files: &[PathBuf]) -> Flocks {
        self.lock(files, Mode::Write).await
    }

    fn try_lock(&self, files: &[PathBuf], mode: Mode) -> Option<Flocks> {
        let mut table = self.table.lock().unwrap();
        let files = table.resolve(files);
        self.start(&mut table, files, mode).ok()
    }

    async fn lock(&self, files: &[PathBuf], mode: Mode) -> Flocks {
        let (wait, ready) = {
            let mut table = self.table.lock().unwrap();
            let files = table.resolve(files);
            let files = match self.start(&mut table, files, mode) {
                Ok(flocks) => return flocks,
                Err(files) => files,
            };
            debug!(
                "failed to get {} locks, waiting",
                match mode {
                    Mode::Read => "read",
                    Mode::Write => "write",
                }
            );
            // Only a release can let this job start, and it wakes the queue.
            let (ready, ready_rx) = oneshot::channel();
            let id = table.enqueue(files.clone(), mode, ready);
            let wait = Wait {
                table: self.table.clone(),
                id,
                mode,
                files,
                granted: false,
            };
            (wait, ready_rx)
        };
        // A waiter leaves the queue only by being granted, which sends, or in
        // `Wait::drop`, so the sender cannot be dropped unsent.
        ready.await.expect("file lock request dropped");
        wait.into_flocks()
    }

    /// Take the locks if no job holding locks conflicts.
    fn start(
        &self,
        table: &mut Table,
        files: Arc<[usize]>,
        mode: Mode,
    ) -> Result<Flocks, Arc<[usize]>> {
        if !table.can_start(&files, mode) {
            return Err(files);
        }
        table.hold(&files, mode);
        Ok(Flocks {
            held: Some(Held {
                table: self.table.clone(),
                mode,
                files,
            }),
        })
    }
}

impl Table {
    fn position(&mut self, file: &PathBuf) -> usize {
        if let Some(&i) = self.index.get(file) {
            return i;
        }
        let i = self.paths.len();
        if self.sorted
            && let Some(&last) = self.order.last()
            && self.paths[last] >= *file
        {
            self.sorted = false;
        }
        self.index.insert(file.clone(), i);
        self.paths.push(file.clone());
        self.order.push(i);
        self.readers.push(0);
        self.writer.push(false);
        i
    }

    /// Positions of `files` without duplicates: a job writing one file twice
    /// would conflict with itself.
    fn resolve(&mut self, files: &[PathBuf]) -> Arc<[usize]> {
        let mut positions = files.iter().map(|f| self.position(f)).collect::<Vec<_>>();
        if !positions.is_sorted_by(|a, b| a < b) {
            positions.sort_unstable();
            positions.dedup();
        }
        positions.into()
    }

    /// Whether nothing holds a lock that conflicts with this request.
    fn can_start(&self, files: &[usize], mode: Mode) -> bool {
        match mode {
            Mode::Read => files.iter().all(|&f| !self.writer[f]),
            Mode::Write => files
                .iter()
                .all(|&f| !self.writer[f] && self.readers[f] == 0),
        }
    }

    fn hold(&mut self, files: &[usize], mode: Mode) {
        for &f in files {
            match mode {
                Mode::Read => self.readers[f] += 1,
                Mode::Write => self.writer[f] = true,
            }
        }
    }

    fn release(&mut self, files: &[usize], mode: Mode) {
        for &f in files {
            match mode {
                Mode::Read => self.readers[f] -= 1,
                Mode::Write => self.writer[f] = false,
            }
        }
    }

    fn enqueue(&mut self, files: Arc<[usize]>, mode: Mode, ready: oneshot::Sender<()>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.queue.push_back(Waiter {
            id,
            mode,
            files,
            ready,
        });
        id
    }

    /// Start, in queue order, every waiting job that conflicts with no holder,
    /// including the jobs this starts.
    fn wake(&mut self) {
        let mut i = 0;
        while i < self.queue.len() {
            let waiter = &self.queue[i];
            if self.can_start(&waiter.files, waiter.mode) {
                let waiter = self.queue.remove(i).unwrap();
                self.hold(&waiter.files, waiter.mode);
                // The receiver lives until `Wait::drop` has taken the request
                // out of the queue, so this cannot fail.
                let _ = waiter.ready.send(());
            } else {
                i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::task::JoinHandle;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    /// Let spawned tasks on this single-threaded runtime run until they wait.
    async fn settle() {
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }

    async fn spawn_lock(
        locks: &Arc<FileRwLocks>,
        mode: Mode,
        files: &[&str],
    ) -> JoinHandle<Flocks> {
        let locks = locks.clone();
        let files = paths(files);
        let handle = tokio::spawn(async move { locks.lock(&files, mode).await });
        settle().await;
        handle
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
    fn duplicate_paths_are_one_lock() {
        let locks = FileRwLocks::new(paths(&["a/b"]));
        // Writing `b` or `a/b` twice would conflict with itself.
        assert!(
            locks
                .try_write(&paths(&["b", "a//b", "b", "a/b"]))
                .is_some()
        );
        assert_eq!(locks.files(), paths(&["a/b", "b"]));
    }

    #[test]
    fn writer_excludes_readers_and_writers() {
        let locks = FileRwLocks::new(paths(&["a", "b"]));
        let write = locks.try_write(&paths(&["b"])).unwrap();
        assert!(locks.try_read(&paths(&["a", "b"])).is_none());
        assert!(locks.try_write(&paths(&["b"])).is_none());
        // A failed attempt takes nothing, so `a` is still free.
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
        let writer = spawn_lock(&locks, Mode::Write, &["b", "a"]).await;
        assert!(!writer.is_finished());
        // The waiting writer holds nothing.
        assert!(!locks.table.lock().unwrap().writer[0]);
        drop(read);
        drop(writer.await.unwrap());
        assert!(locks.try_write(&paths(&["a", "b"])).is_some());
    }

    /// The case that serialized prettier, tsc and prettier again: a fixer `e`
    /// holds `p`, `q` and `r`; two batches of another fixer wait for `r` and
    /// for `p` and `q`; a type checker waits to read `q` and `r`. When `e`
    /// finishes, both batches run together and the checker waits for them.
    /// Taking locks one file at a time, the checker would take `q` while it
    /// waited for `r`, and the second batch would wait for the checker.
    #[tokio::test]
    async fn waiting_reader_does_not_hold_back_writers() {
        let locks = Arc::new(FileRwLocks::new(paths(&["p", "q", "r"])));
        let e = locks.try_write(&paths(&["p", "q", "r"])).unwrap();
        let batch1 = spawn_lock(&locks, Mode::Write, &["r"]).await;
        let batch2 = spawn_lock(&locks, Mode::Write, &["p", "q"]).await;
        let checker = spawn_lock(&locks, Mode::Read, &["q", "r"]).await;
        drop(e);
        settle().await;
        assert!(batch1.is_finished());
        assert!(batch2.is_finished());
        assert!(!checker.is_finished());
        drop(batch1.await.unwrap());
        settle().await;
        assert!(!checker.is_finished());
        drop(batch2.await.unwrap());
        drop(checker.await.unwrap());
    }

    /// A waiting job holds up no one: a writer of every file waits for a
    /// writer of one, and meanwhile jobs using the other files start.
    #[tokio::test]
    async fn waiting_job_does_not_block_later_ones() {
        let locks = Arc::new(FileRwLocks::new(paths(&["a", "b", "c"])));
        let a = locks.try_write(&paths(&["a"])).unwrap();
        let all = spawn_lock(&locks, Mode::Write, &["a", "b", "c"]).await;
        let b = locks.try_write(&paths(&["b"])).unwrap();
        let c = spawn_lock(&locks, Mode::Read, &["c"]).await;
        assert!(c.is_finished());
        drop(a);
        settle().await;
        assert!(!all.is_finished());
        drop(b);
        settle().await;
        assert!(!all.is_finished());
        drop(c.await.unwrap());
        drop(all.await.unwrap());
    }

    /// When a release frees files that several waiting jobs want, the one
    /// that asked first starts.
    #[tokio::test]
    async fn first_waiter_goes_first() {
        let locks = Arc::new(FileRwLocks::new(paths(&["a"])));
        let a = locks.try_write(&paths(&["a"])).unwrap();
        let writer = spawn_lock(&locks, Mode::Write, &["a"]).await;
        let reader = spawn_lock(&locks, Mode::Read, &["a"]).await;
        drop(a);
        settle().await;
        assert!(writer.is_finished());
        assert!(!reader.is_finished());
        drop(writer.await.unwrap());
        drop(reader.await.unwrap());
    }

    /// Readers queued behind a writer start together once it finishes.
    #[tokio::test]
    async fn queued_readers_share() {
        let locks = Arc::new(FileRwLocks::new(paths(&["a"])));
        let write = locks.try_write(&paths(&["a"])).unwrap();
        let first = spawn_lock(&locks, Mode::Read, &["a"]).await;
        let second = spawn_lock(&locks, Mode::Read, &["a"]).await;
        drop(write);
        settle().await;
        assert!(first.is_finished());
        assert!(second.is_finished());
    }

    /// A cancelled request leaves the queue, so no release grants it.
    #[tokio::test]
    async fn cancelled_request_leaves_queue() {
        let locks = Arc::new(FileRwLocks::new(paths(&["a", "b"])));
        let a = locks.try_write(&paths(&["a"])).unwrap();
        let writer = spawn_lock(&locks, Mode::Write, &["a", "b"]).await;
        writer.abort();
        assert!(writer.await.unwrap_err().is_cancelled());
        assert!(locks.table.lock().unwrap().queue.is_empty());
        drop(a);
        assert!(locks.try_write(&paths(&["a", "b"])).is_some());
    }

    /// A request cancelled after being granted, before its task saw the
    /// grant, releases its locks.
    #[tokio::test]
    async fn cancelled_after_grant_releases() {
        let locks = Arc::new(FileRwLocks::new(paths(&["a"])));
        let a = locks.try_write(&paths(&["a"])).unwrap();
        let writer = spawn_lock(&locks, Mode::Write, &["a"]).await;
        // Granted to the waiting task, which has not run since.
        drop(a);
        assert!(locks.try_read(&paths(&["a"])).is_none());
        writer.abort();
        assert!(writer.await.unwrap_err().is_cancelled());
        assert!(locks.try_write(&paths(&["a"])).is_some());
    }
}
