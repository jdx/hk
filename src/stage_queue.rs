//! One `git add` for every step that stages at the same time.
//!
//! Steps that finish together (typically because racily clean files make each
//! wait for every other step's writes) would otherwise run one `git add` each,
//! one after another, and each rewrites the whole index.

use crate::Result;
use indexmap::IndexSet;
use itertools::Itertools;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use tokio::sync::{Mutex, MutexGuard};

/// Files queued for staging that no `git add` has staged yet.
#[derive(Default)]
pub struct StageQueue {
    pending: StdMutex<Pending>,
}

#[derive(Default)]
struct Pending {
    paths: IndexSet<PathBuf>,
    /// Set by the caller that stages the queue: `Err` holds its error message.
    result: Arc<OnceLock<std::result::Result<(), String>>>,
}

impl StageQueue {
    /// Stages `paths` together with the files other callers queue meanwhile,
    /// through a single call to `add`, and returns once they are staged.
    ///
    /// `guard` must be held on `lock`, the mutex every caller stages under, and
    /// is released while the others queue. Files another caller already
    /// queued are removed from `paths`, as if its `add` had already run. If
    /// the shared `add` fails, every caller whose files it held gets the error.
    pub async fn stage<G>(
        &self,
        lock: &Mutex<G>,
        guard: MutexGuard<'_, G>,
        paths: &mut Vec<PathBuf>,
        add: impl FnOnce(&G, &[PathBuf]) -> Result<()>,
    ) -> Result<()> {
        let result = {
            let mut pending = self.pending.lock().unwrap();
            // That `add` has not run yet, so it stages each file as it is now.
            paths.retain(|p| !pending.paths.contains(p));
            pending.paths.extend(paths.iter().cloned());
            pending.result.clone()
        };
        // Let the callers waiting for the mutex queue their files, then stage
        // the queue unless one of them already has. The mutex is fair, so the
        // first caller to queue usually gets it back after the others queued.
        drop(guard);
        let guard = lock.lock().await;
        if result.get().is_none() {
            let pending = std::mem::take(&mut *self.pending.lock().unwrap());
            // Only staging the queue replaces it, so it is still ours.
            debug_assert!(Arc::ptr_eq(&pending.result, &result));
            let paths = pending.paths.into_iter().collect_vec();
            let res = add(&guard, &paths);
            let _ = pending
                .result
                .set(res.as_ref().map(|_| ()).map_err(|e| format!("{e:#}")));
            return res;
        }
        drop(guard);
        match result.get() {
            Some(Err(err)) => Err(eyre::eyre!("{err}")),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type Adds = Arc<StdMutex<Vec<Vec<PathBuf>>>>;

    fn paths(names: &[&str]) -> Vec<PathBuf> {
        names.iter().map(PathBuf::from).collect()
    }

    /// Runs one `stage` per entry of `requests` at the same time: every task
    /// is waiting for the mutex before any gets it. Returns what each task
    /// kept, what each got, and each `add` call's files.
    async fn stage_together(
        requests: Vec<Vec<PathBuf>>,
        fail: bool,
    ) -> (Vec<Vec<PathBuf>>, Vec<Result<()>>, Vec<Vec<PathBuf>>) {
        let queue = Arc::new(StageQueue::default());
        let lock = Arc::new(Mutex::new(()));
        let adds: Adds = Default::default();
        let waiting = Arc::new(AtomicUsize::new(0));
        let held = lock.lock().await;
        let n = requests.len();
        let tasks = requests
            .into_iter()
            .map(|mut request| {
                let (queue, lock, adds, waiting) =
                    (queue.clone(), lock.clone(), adds.clone(), waiting.clone());
                tokio::spawn(async move {
                    waiting.fetch_add(1, Ordering::SeqCst);
                    let guard = lock.lock().await;
                    let res = queue
                        .stage(&lock, guard, &mut request, |_, p| {
                            adds.lock().unwrap().push(p.to_vec());
                            if fail {
                                Err(eyre::eyre!("simulated add failure"))
                            } else {
                                Ok(())
                            }
                        })
                        .await;
                    (request, res)
                })
            })
            .collect_vec();
        // The tests run on a single-threaded runtime, so every task has
        // started waiting for the mutex once each has run up to it.
        while waiting.load(Ordering::SeqCst) < n {
            tokio::task::yield_now().await;
        }
        tokio::task::yield_now().await;
        drop(held);
        let mut kept = vec![];
        let mut results = vec![];
        for task in tasks {
            let (request, res) = task.await.unwrap();
            kept.push(request);
            results.push(res);
        }
        let adds = adds.lock().unwrap().clone();
        (kept, results, adds)
    }

    #[tokio::test]
    async fn callers_staging_together_share_one_add() {
        let (kept, results, adds) = stage_together(
            vec![paths(&["a"]), paths(&["b", "c"]), paths(&["d"])],
            false,
        )
        .await;
        assert!(results.iter().all(|r| r.is_ok()));
        assert_eq!(adds, vec![paths(&["a", "b", "c", "d"])]);
        assert_eq!(kept, vec![paths(&["a"]), paths(&["b", "c"]), paths(&["d"])]);
    }

    #[tokio::test]
    async fn a_file_is_kept_by_the_first_caller_to_queue_it() {
        let (kept, results, adds) =
            stage_together(vec![paths(&["a", "b"]), paths(&["b", "c"])], false).await;
        assert!(results.iter().all(|r| r.is_ok()));
        assert_eq!(adds, vec![paths(&["a", "b", "c"])]);
        assert_eq!(kept, vec![paths(&["a", "b"]), paths(&["c"])]);
    }

    #[tokio::test]
    async fn a_failed_add_fails_every_caller_in_it() {
        let (_, results, adds) =
            stage_together(vec![paths(&["a"]), paths(&["b"]), paths(&["c"])], true).await;
        assert_eq!(adds.len(), 1);
        for res in results {
            let err = res.unwrap_err();
            assert!(format!("{err}").contains("simulated add failure"), "{err}");
        }
    }

    #[tokio::test]
    async fn callers_staging_one_after_another_each_add() {
        let queue = StageQueue::default();
        let lock = Mutex::new(());
        let mut adds = vec![];
        for request in [paths(&["a"]), paths(&["a", "b"])] {
            let mut request = request;
            let guard = lock.lock().await;
            queue
                .stage(&lock, guard, &mut request, |_, p| {
                    adds.push(p.to_vec());
                    Ok(())
                })
                .await
                .unwrap();
        }
        // The first `add` already ran, so the second caller keeps "a".
        assert_eq!(adds, vec![paths(&["a"]), paths(&["a", "b"])]);
    }
}
