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
    /// `shared_mode` is the permission mode to give a newly created file in a
    /// shared repository (see [`shared_repository_mode`]); `None` keeps the
    /// process umask. An existing file is opened read-only, which is enough to
    /// lock it, so accounts that cannot write to it can still stash.
    ///
    /// If another process holds it, calls `on_wait` once and keeps trying
    /// until `timeout` has passed. A zero timeout fails at once. If `cancel`
    /// fires while waiting, fails with [`Cancelled`] within one poll interval.
    pub fn acquire(
        path: &Path,
        shared_mode: Option<u32>,
        timeout: Duration,
        cancel: &CancellationToken,
        on_wait: impl FnOnce(),
    ) -> Result<Self> {
        let file = open_lock_file(path, shared_mode)
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

/// Map the value of `core.sharedRepository` to the mode a file hk creates in
/// the repository should get, or `None` when the umask should apply.
///
/// Mirrors git's file modes: `group`/`true`/`1` is group-writable (0660),
/// `all`/`world`/`everybody`/`2` is group-writable and readable by everyone
/// (0664, not world-writable), `umask`/`false`/`0`/unset is the umask, and an
/// octal value gets owner read/write added and the execute bits cleared.
pub fn shared_repository_mode(value: Option<&str>) -> Option<u32> {
    let value = value?.trim().to_ascii_lowercase();
    match value.as_str() {
        "" | "umask" | "false" | "no" | "off" | "0" => None,
        "group" | "true" | "yes" | "on" | "1" => Some(0o660),
        "all" | "world" | "everybody" | "2" => Some(0o664),
        v if v.starts_with('0') => u32::from_str_radix(v, 8)
            .ok()
            .filter(|m| *m <= 0o777)
            .map(|m| (m | 0o600) & !0o111),
        _ => None,
    }
}

/// Opens (creating if needed) a lock file in the repository: an existing one
/// read-only, a new one with `shared_mode` when set. Shared with the journal's
/// lock so both honor `core.sharedRepository` alike.
pub fn open_lock_file(path: &Path, shared_mode: Option<u32>) -> std::io::Result<File> {
    // An existing file only needs to be readable to be locked, so a lock file
    // another account created without group write still works for us.
    match OpenOptions::new().read(true).open(path) {
        Ok(file) => return Ok(file),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    let mut opts = OpenOptions::new();
    opts.create(true).truncate(false).read(true).write(true);
    shared_create_mode(&mut opts, shared_mode);
    let file = opts.open(path)?;
    apply_shared_mode(&file, shared_mode);
    Ok(file)
}

/// Asks `opts` to create a file with `shared_mode` (see
/// [`shared_repository_mode`]) when set; the umask still applies to it, so
/// follow with [`apply_shared_mode`] on the opened file. Every file hk creates
/// in the shared git directory goes through both, so `core.sharedRepository`
/// holds for all of them alike.
pub fn shared_create_mode(opts: &mut OpenOptions, shared_mode: Option<u32>) {
    #[cfg(unix)]
    if let Some(mode) = shared_mode {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = (opts, shared_mode);
}

/// Gives a file hk just created `shared_mode` when set, undoing what the
/// umask stripped from the create mode. Best effort: a failure leaves the
/// umask's result.
pub fn apply_shared_mode(file: &File, shared_mode: Option<u32>) {
    #[cfg(unix)]
    if let Some(mode) = shared_mode {
        use std::os::unix::fs::PermissionsExt;
        let _ = file.set_permissions(std::fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    let _ = (file, shared_mode);
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
            None,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        let waited = Arc::new(AtomicBool::new(false));
        let flag = waited.clone();
        let err = StashLock::acquire(
            &path,
            None,
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
            None,
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
                None,
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
            None,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        drop(held);
        StashLock::acquire(
            &path,
            None,
            Duration::ZERO,
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
    }

    #[test]
    fn waiter_gets_the_lock_once_the_holder_drops() {
        let (_dir, path) = lock_path();
        let held = StashLock::acquire(
            &path,
            None,
            Duration::from_secs(1),
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        let waiter_path = path.clone();
        let waiter = thread::spawn(move || {
            StashLock::acquire(
                &waiter_path,
                None,
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
            None,
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
        let err =
            StashLock::acquire(&path, None, Duration::from_secs(30), &cancel, || {}).unwrap_err();
        assert!(err.downcast_ref::<Cancelled>().is_some(), "{err:#}");
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn shared_repository_values_map_to_modes() {
        assert_eq!(shared_repository_mode(None), None);
        assert_eq!(shared_repository_mode(Some("umask")), None);
        assert_eq!(shared_repository_mode(Some("false")), None);
        assert_eq!(shared_repository_mode(Some("0")), None);
        assert_eq!(shared_repository_mode(Some("group")), Some(0o660));
        assert_eq!(shared_repository_mode(Some("true")), Some(0o660));
        assert_eq!(shared_repository_mode(Some("1")), Some(0o660));
        assert_eq!(shared_repository_mode(Some("all")), Some(0o664));
        assert_eq!(shared_repository_mode(Some("World")), Some(0o664));
        assert_eq!(shared_repository_mode(Some("2")), Some(0o664));
        assert_eq!(shared_repository_mode(Some("0640")), Some(0o640 | 0o600));
        assert_eq!(shared_repository_mode(Some("0440")), Some(0o640));
        assert_eq!(shared_repository_mode(Some("0777")), Some(0o666));
        assert_eq!(shared_repository_mode(Some("0750")), Some(0o640));
        assert_eq!(shared_repository_mode(Some("nonsense")), None);
    }

    #[cfg(unix)]
    #[test]
    fn read_only_lock_file_can_still_be_locked() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, path) = lock_path();
        std::fs::write(&path, "").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
        let held = StashLock::acquire(
            &path,
            None,
            Duration::ZERO,
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        assert!(
            StashLock::acquire(
                &path,
                None,
                Duration::ZERO,
                &CancellationToken::new(),
                || {}
            )
            .is_err()
        );
        drop(held);
    }

    #[cfg(unix)]
    #[test]
    fn shared_mode_beats_the_umask_on_creation() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, path) = lock_path();
        let _held = StashLock::acquire(
            &path,
            Some(0o660),
            Duration::ZERO,
            &CancellationToken::new(),
            || {},
        )
        .unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o660);
    }
}
