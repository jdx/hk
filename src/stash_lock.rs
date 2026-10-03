//! A lock that keeps one hk process from stashing while another has stashed.
//!
//! Every worktree of a repository shares one stash stack, and hk finds the
//! entry it made again when it restores. Two hk processes that each stash and
//! restore at the same time can restore each other's changes or drop the wrong
//! entry. The in-memory locks of a hook run cannot help, as they live in one
//! process, so this takes an advisory lock on a file in the repository's
//! common git directory, which every linked worktree shares.
//!
//! The operating system releases the lock when the holder exits, including
//! when it is killed, so a crash never leaves it held.

use std::{
    fs::{File, OpenOptions, TryLockError},
    path::Path,
    thread,
    time::{Duration, Instant},
};

use eyre::{Result, WrapErr, eyre};
use tokio_util::sync::CancellationToken;

const POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Name of the lock file inside the common git directory.
pub const LOCK_FILE_NAME: &str = "hk-stash.lock";

/// Returned (inside the `eyre::Report`) when `cancel` fired while waiting.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cancelled while waiting for the stash lock")
    }
}

impl std::error::Error for Cancelled {}

/// Holds the lock until dropped.
#[derive(Debug)]
pub struct StashLock {
    file: File,
}

impl StashLock {
    /// Take the lock on `path`, creating the file if needed.
    ///
    /// If another process holds it, calls `on_wait` once and keeps trying
    /// until `timeout` has passed. A zero timeout fails at once. If `cancel`
    /// fires while waiting, fails with [`Cancelled`] within one poll interval.
    pub fn acquire(
        path: &Path,
        timeout: Duration,
        cancel: &CancellationToken,
        on_wait: impl FnOnce(),
    ) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .wrap_err_with(|| format!("failed to open stash lock {}", path.display()))?;
        let start = Instant::now();
        let mut on_wait = Some(on_wait);
        loop {
            match file.try_lock() {
                Ok(()) => {
                    return Ok(Self { file });
                }
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(err)) => {
                    return Err(err).wrap_err_with(|| format!("failed to lock {}", path.display()));
                }
            }
            if cancel.is_cancelled() {
                return Err(Cancelled.into());
            }
            if start.elapsed() >= timeout {
                return Err(eyre!(
                    "timed out after {}s waiting for another hk process to finish stashing; \
                     it holds the lock {}. Wait for it to finish, or raise \
                     HK_STASH_LOCK_TIMEOUT (seconds) / `git config hk.stashLockTimeout`",
                    timeout.as_secs(),
                    path.display()
                ));
            }
            if let Some(on_wait) = on_wait.take() {
                on_wait();
            }
            thread::sleep(POLL_INTERVAL);
        }
    }
}

impl Drop for StashLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    fn lock_path() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOCK_FILE_NAME);
        (dir, path)
    }

    #[test]
    fn second_acquire_times_out_naming_the_path() {
        let (_dir, path) = lock_path();
        let _held = StashLock::acquire(
            &path,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        let waited = Arc::new(AtomicBool::new(false));
        let flag = waited.clone();
        let err = StashLock::acquire(
            &path,
            Duration::from_millis(150),
            &CancellationToken::new(),
            move || flag.store(true, Ordering::SeqCst),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("timed out"), "{err}");
        assert!(err.contains(&path.display().to_string()), "{err}");
        assert!(waited.load(Ordering::SeqCst), "on_wait was not called");
    }

    #[test]
    fn zero_timeout_fails_without_waiting() {
        let (_dir, path) = lock_path();
        let _held = StashLock::acquire(
            &path,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        let waited = Arc::new(AtomicBool::new(false));
        let flag = waited.clone();
        let start = Instant::now();
        assert!(
            StashLock::acquire(
                &path,
                Duration::ZERO,
                &CancellationToken::new(),
                move || { flag.store(true, Ordering::SeqCst) }
            )
            .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(1));
        assert!(!waited.load(Ordering::SeqCst));
    }

    #[test]
    fn drop_releases_the_lock() {
        let (_dir, path) = lock_path();
        let held = StashLock::acquire(
            &path,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        drop(held);
        StashLock::acquire(&path, Duration::ZERO, &CancellationToken::new(), || {}).unwrap();
    }

    #[test]
    fn waiter_gets_the_lock_once_the_holder_drops() {
        let (_dir, path) = lock_path();
        let held = StashLock::acquire(
            &path,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        let waiter_path = path.clone();
        let waiter = thread::spawn(move || {
            StashLock::acquire(
                &waiter_path,
                Duration::from_secs(10),
                &CancellationToken::new(),
                || {},
            )
            .map(|_| ())
        });
        thread::sleep(Duration::from_millis(150));
        assert!(!waiter.is_finished());
        drop(held);
        waiter.join().unwrap().unwrap();
    }

    #[test]
    fn cancel_stops_the_wait_promptly() {
        let (_dir, path) = lock_path();
        let _held = StashLock::acquire(
            &path,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        let cancel = CancellationToken::new();
        let trigger = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            trigger.cancel();
        });
        let start = Instant::now();
        let err = StashLock::acquire(&path, Duration::from_secs(30), &cancel, || {}).unwrap_err();
        assert!(err.downcast_ref::<Cancelled>().is_some(), "{err:#}");
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
