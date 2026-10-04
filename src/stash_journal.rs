//! The pending-stash journal.
//!
//! While hk has the user's unstaged changes stashed, a small JSON file in the
//! repository's git directory says so: which stash entries hk made, which
//! process owns them, and which hook it was running. hk removes the file once
//! the changes are back. If hk is killed in between (SIGKILL, power loss, an
//! unhandled console event), the file is what the next run finds, so the
//! changes are restored or reported instead of silently staying in the stash.
//!
//! This module holds the file format, the liveness check for the owning
//! process and the decision of what to do with a journal that was left
//! behind. The git operations that act on that decision live in `git.rs`.

use std::path::{Path, PathBuf};

use eyre::{Result, WrapErr, eyre};
use serde::{Deserialize, Serialize};

/// Name of the journal inside the git directory of the worktree.
pub const FILE_NAME: &str = "hk-pending-stash";

const VERSION: u32 = 1;

/// What a stash entry holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StashKind {
    /// The unstaged (and, with `HK_STASH_UNTRACKED`, untracked) changes.
    Unstaged,
    /// Contents of intent-to-add files, held in an entry of their own.
    IntentToAdd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    pub commit: String,
    pub kind: StashKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub version: u32,
    /// The hk process that owns the stash entries.
    pub pid: u32,
    /// When that process started, to tell it from a later process that reuses
    /// the pid. `None` where hk cannot read it.
    pub pid_start: Option<String>,
    /// Which machine and PID namespace the owner ran in (see
    /// [`host_identity`]). The git directory can be shared by hosts or
    /// containers that cannot see each other's processes, so a pid that is not
    /// running here proves nothing unless this matches. `None` in journals
    /// written by older hk versions and where it cannot be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_host: Option<String>,
    /// Seconds since the Unix epoch when the journal was written.
    pub timestamp: u64,
    pub hook: String,
    pub worktree: String,
    /// The stash entries that existed before hk pushed its own. Entries hk
    /// created but died before recording are found by their absence here.
    pub stashes_before: Vec<String>,
    /// The entries hk has created so far.
    pub entries: Vec<JournalEntry>,
}

impl Journal {
    pub fn new(hook: &str, worktree: &Path, stashes_before: Vec<String>) -> Self {
        let pid = std::process::id();
        Self {
            version: VERSION,
            pid,
            pid_start: process_start_token(pid),
            owner_host: host_identity(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or_default(),
            hook: hook.to_string(),
            worktree: worktree.display().to_string(),
            stashes_before,
            entries: vec![],
        }
    }

    pub fn parse(text: &str) -> Result<Self> {
        let journal: Self = serde_json::from_str(text).wrap_err("not valid hk journal JSON")?;
        if journal.version != VERSION {
            return Err(eyre!(
                "journal version {} is not understood (this hk reads version {VERSION})",
                journal.version
            ));
        }
        Ok(journal)
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    /// Whether the owner of this journal is gone, running or cannot be told.
    /// `here` is the identity of the current process (see [`host_identity`]),
    /// `alive` whether a process with `self.pid` exists in this namespace and
    /// `current_start` that process's start token.
    ///
    /// A pid that is not running only means the owner is gone when the journal
    /// was written where hk is running now. From another host or PID namespace
    /// the owner may well be running, so nothing is assumed.
    pub fn owner(&self, here: Option<&str>, alive: bool, current_start: Option<&str>) -> Owner {
        let start_differs =
            matches!((self.pid_start.as_deref(), current_start), (Some(a), Some(b)) if a != b);
        match (self.owner_host.as_deref(), here) {
            (Some(recorded), Some(here)) if recorded == here => {
                if !alive || start_differs {
                    Owner::Gone
                } else {
                    Owner::Running
                }
            }
            (Some(_), Some(_)) => Owner::Unknown(
                "it was written by another host or PID namespace, whose processes hk cannot see",
            ),
            (Some(_), None) => Owner::Unknown(
                "hk cannot tell which host and PID namespace it is running in to compare with the journal",
            ),
            // Written by an older hk, which did not record it: a pid that is
            // visible here is judged as before, one that is not proves nothing
            (None, _) if alive => {
                if start_differs {
                    Owner::Gone
                } else {
                    Owner::Running
                }
            }
            (None, _) => Owner::Unknown(
                "it does not record which host or PID namespace it was written in, and its pid is not running here",
            ),
        }
    }

    /// [`Self::owner`] judged from this process.
    pub fn owner_now(&self) -> Owner {
        self.owner(
            host_identity().as_deref(),
            process_is_alive(self.pid),
            process_start_token(self.pid).as_deref(),
        )
    }

    /// The owner, as the journal records it, for messages.
    pub fn describe(&self) -> String {
        let when = chrono::DateTime::from_timestamp(self.timestamp as i64, 0)
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| self.timestamp.to_string());
        format!(
            "hk (pid {}, hook `{}`, started {when})",
            self.pid, self.hook
        )
    }
}

/// What a journal's owner is doing, as far as hk can tell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// The process that wrote it no longer exists, where it ran.
    Gone,
    /// Still running here.
    Running,
    /// hk cannot see the owner's processes: nothing may be restored or dropped.
    Unknown(&'static str),
}

/// Identifies where this process runs, so that a journal in a git directory
/// shared by several hosts (a network file system) or PID namespaces (a
/// repository mounted into a container) is only judged by a process that can
/// see the owner. The host name, and on Linux the PID namespace.
pub fn host_identity() -> Option<String> {
    let host = hostname()?;
    #[cfg(target_os = "linux")]
    {
        let ns = std::fs::read_link("/proc/self/ns/pid").ok()?;
        Some(format!("{host}|{}", ns.display()))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Some(host)
    }
}

fn hostname() -> Option<String> {
    #[cfg(unix)]
    {
        let mut buf = [0u8; 256];
        // SAFETY: the buffer is valid for its length, which is passed
        let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
        if rc != 0 {
            return None;
        }
        let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
        let name = String::from_utf8_lossy(&buf[..end]).trim().to_string();
        (!name.is_empty()).then_some(name)
    }
    #[cfg(windows)]
    {
        std::env::var("COMPUTERNAME")
            .ok()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
    }
}

/// Name of the lock file that serializes every operation on the journal. It is
/// never deleted: the OS releases the lock when its holder exits or is killed,
/// and an empty file left behind carries no state.
pub const LOCK_NAME: &str = "hk-pending-stash.lock";

/// How long an operation waits for the journal lock. Holders keep it for a few
/// milliseconds, except a recovering run, which keeps it while it restores.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(15);

/// Exclusive hold on the journal of one worktree, taken on a dedicated lock
/// file next to it. Every create, update, remove and recovery happens while it
/// is held, so no two hk processes act on the journal at once. Dropping it, or
/// the process dying, releases it.
#[derive(Debug)]
pub struct JournalLock {
    _file: std::fs::File,
}

impl JournalLock {
    /// Waits for the lock next to the journal at `path`. `Ok(None)` when the
    /// wait ran out or hk was told to shut down meanwhile; the caller then
    /// leaves the journal alone.
    ///
    /// The lock file is opened like the stash lock's: an existing one
    /// read-only, which is enough to lock it, and a new one with the mode
    /// `core.sharedRepository` asks for (`shared_mode`), so a file another
    /// account created never locks this one out.
    pub fn acquire(path: &Path, shared_mode: Option<u32>) -> Result<Option<Self>> {
        Self::acquire_for(path, LOCK_WAIT, shared_mode)
    }

    fn acquire_for(
        path: &Path,
        wait: std::time::Duration,
        shared_mode: Option<u32>,
    ) -> Result<Option<Self>> {
        use std::fs::TryLockError;
        let lock_path = path.with_file_name(LOCK_NAME);
        let file = crate::stash_lock::open_lock_file(&lock_path, shared_mode)
            .wrap_err_with(|| format!("failed to open {}", lock_path.display()))?;
        let deadline = std::time::Instant::now() + wait;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Some(Self { _file: file })),
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(err)) => {
                    return Err(err)
                        .wrap_err_with(|| format!("failed to lock {}", lock_path.display()));
                }
            }
            if std::time::Instant::now() >= deadline || crate::shutdown::exit_code().is_some() {
                return Ok(None);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

/// Reads the journal at `path`. `Ok(None)` when there is none.
pub fn read(path: &Path) -> Result<Option<Journal>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(Journal::parse(&text)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).wrap_err_with(|| format!("failed to read {}", path.display())),
    }
}

fn is_temp_name(name: &str) -> bool {
    name.starts_with(&format!("{FILE_NAME}.")) && name.ends_with(".tmp")
}

/// Deletes temporary files that a process which died mid-write left behind.
/// Temporary files are only written while the lock is held, so under the lock
/// every one that exists is such a leftover. Needs the proof of the lock.
pub fn sweep_temp_files(path: &Path, _lock: &JournalLock) {
    let Some(dir) = path.parent() else { return };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_str().is_some_and(is_temp_name) {
            remove_file(&entry.path());
        }
    }
}

/// Writes `journal` to a temporary file that no one else has, and flushes it.
/// The name carries a time and a counter besides the pid, and the file is
/// created with `create_new`, so an existing file under that name (one a
/// crashed process left, even one that shares an inode with the journal) is
/// never opened, let alone truncated: a taken name is skipped.
fn write_temp(path: &Path, journal: &Journal, _lock: &JournalLock) -> Result<PathBuf> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let json = journal.to_json()?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    for _ in 0..100 {
        let tmp = path.with_file_name(format!(
            "{FILE_NAME}.{}-{nanos:x}-{}.tmp",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(err).wrap_err_with(|| format!("failed to write {}", tmp.display()));
            }
        };
        let written = file
            .write_all(json.as_bytes())
            // On disk before it is renamed in, or power loss could leave a
            // journal whose content never made it
            .and_then(|()| file.sync_all());
        return match written {
            Ok(()) => Ok(tmp),
            Err(err) => {
                let _ = std::fs::remove_file(&tmp);
                Err(err).wrap_err_with(|| format!("failed to write {}", tmp.display()))
            }
        };
    }
    Err(eyre!(
        "no free temporary file name next to {}",
        path.display()
    ))
}

/// Flushes the directory entry of a created, renamed or removed file. Windows
/// offers no portable way to sync a directory, so it relies on NTFS metadata
/// journaling there.
fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent()
        && let Ok(dir) = std::fs::File::open(dir)
        && let Err(err) = dir.sync_all()
    {
        log::debug!("failed to sync {}: {err}", path.display());
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Writes `journal` at `path` only if nothing is there: `Ok(false)` leaves the
/// file that is, whatever it holds. Under the lock no one else can create it
/// between the check and the rename.
fn create(path: &Path, journal: &Journal, lock: &JournalLock) -> Result<bool> {
    sweep_temp_files(path, lock);
    if path
        .try_exists()
        .wrap_err_with(|| format!("failed to check {}", path.display()))?
    {
        return Ok(false);
    }
    replace(path, journal, lock)?;
    Ok(true)
}

/// Replaces the journal at `path` atomically: readers see the old or the new
/// content, never a partial one.
fn replace(path: &Path, journal: &Journal, lock: &JournalLock) -> Result<()> {
    let tmp = write_temp(path, journal, lock)?;
    // Windows refuses to replace a file that a scanner or indexer has open for
    // a moment, so a denied rename is tried again
    let mut attempt = 0;
    loop {
        match std::fs::rename(&tmp, path) {
            Ok(()) => break,
            Err(err)
                if cfg!(windows)
                    && err.kind() == std::io::ErrorKind::PermissionDenied
                    && attempt < 20 =>
            {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(err) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(err).wrap_err_with(|| format!("failed to write {}", path.display()));
            }
        }
    }
    sync_parent(path);
    Ok(())
}

/// Deletes the journal at `path` and flushes the directory. Needs the lock.
pub fn discard(path: &Path, _lock: &JournalLock) {
    remove_file(path);
    sync_parent(path);
}

/// The journal this process owns while it has changes stashed. Each operation
/// takes the journal lock for its own short critical section; the lock is not
/// held in between.
#[derive(Debug)]
pub struct OwnedJournal {
    path: PathBuf,
    journal: Journal,
    shared_mode: Option<u32>,
}

impl OwnedJournal {
    /// Writes a journal with no entries yet, before the worktree is touched.
    /// `Ok(None)` when a journal already exists, whose owner or leftovers hk
    /// must not overwrite.
    pub fn begin(
        path: PathBuf,
        journal: Journal,
        shared_mode: Option<u32>,
    ) -> Result<Option<Self>> {
        let lock = Self::lock(&path, shared_mode)?;
        Ok(create(&path, &journal, &lock)?.then_some(Self {
            path,
            journal,
            shared_mode,
        }))
    }

    fn lock(path: &Path, shared_mode: Option<u32>) -> Result<JournalLock> {
        JournalLock::acquire(path, shared_mode)?.ok_or_else(|| {
            eyre!(
                "gave up waiting for another hk to finish with {}",
                path.display()
            )
        })
    }

    pub fn has_entries(&self) -> bool {
        !self.journal.entries.is_empty()
    }

    /// Records a stash entry hk created.
    pub fn record(&mut self, commit: &str, kind: StashKind) -> Result<()> {
        if self.journal.entries.iter().any(|e| e.commit == commit) {
            return Ok(());
        }
        let lock = Self::lock(&self.path, self.shared_mode)?;
        self.journal.entries.push(JournalEntry {
            commit: commit.to_string(),
            kind,
        });
        replace(&self.path, &self.journal, &lock)
    }

    /// Forgets an entry hk dropped itself.
    pub fn forget(&mut self, commit: &str) -> Result<()> {
        let lock = Self::lock(&self.path, self.shared_mode)?;
        self.journal.entries.retain(|e| e.commit != commit);
        replace(&self.path, &self.journal, &lock)
    }

    /// Deletes the journal: the changes are back in the worktree. When the
    /// lock cannot be had the journal stays, which is harmless: the next run
    /// finds no entry of it in the stash and discards it.
    pub fn remove(self) {
        let lock = match Self::lock(&self.path, self.shared_mode) {
            Ok(lock) => lock,
            Err(err) => {
                log::warn!("left the pending-stash journal in place: {err}");
                return;
            }
        };
        // Only a journal that is still this process's: nobody else writes
        // there while it lives, but a deletion must never take another's
        match read(&self.path) {
            Ok(Some(on_disk)) if on_disk.pid != self.journal.pid => {
                log::warn!(
                    "{} belongs to another hk now; leaving it",
                    self.path.display()
                );
            }
            _ => discard(&self.path, &lock),
        }
    }
}

pub fn remove_file(path: &Path) {
    if let Err(err) = std::fs::remove_file(path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("failed to remove {}: {err}", path.display());
    }
}

/// A stash list entry, as the decision needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashRow {
    pub commit: String,
    pub subject: String,
}

/// The text after `: ` or at the start of a stash subject, which git writes as
/// `On <branch>: <message>`.
fn stash_message(subject: &str) -> &str {
    subject
        .split_once(": ")
        .map(|(_, rest)| rest)
        .filter(|rest| rest.starts_with("hk: "))
        .unwrap_or(subject)
}

/// The pid in the message hk gives its stash entries, `hk: <pid>-<nanos>-<n>`.
fn message_pid(subject: &str) -> Option<u32> {
    let rest = stash_message(subject).strip_prefix("hk: ")?;
    rest.split('-').next()?.parse().ok()
}

/// Whether a stash subject is the fixed `hk: intent-to-add files` message that
/// earlier hk versions gave that entry, which has no pid. Current hk gives it
/// a per-run message like every other entry.
fn is_intent_to_add(subject: &str) -> bool {
    const INTENT_TO_ADD: &str = "hk: intent-to-add files";
    stash_message(subject) == INTENT_TO_ADD
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportReason {
    /// The worktree has changes of its own that applying could collide with.
    DirtyWorktree,
    /// A stash entry that looks like hk's was created and never recorded, so
    /// hk cannot tell whether it is its own.
    UnrecordedEntry,
    /// The journal names more than one entry of a kind, which hk never makes.
    Ambiguous,
    /// The owner may still be running, in another host or PID namespace.
    UnknownOwner(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Nothing of the user's is left in the stash: delete the journal.
    Discard(&'static str),
    /// Restore these entries, newest first as recorded, then delete the journal.
    Restore(Vec<JournalEntry>),
    /// Leave everything alone and tell the user how to recover.
    Report {
        reason: ReportReason,
        commits: Vec<String>,
    },
}

/// What to do with a journal whose owner is gone. `stash` is the stash list
/// now, and `worktree_clean` whether the worktree has no change that is not
/// staged and no untracked file.
pub fn decide(journal: &Journal, stash: &[StashRow], worktree_clean: bool) -> Action {
    let present: Vec<JournalEntry> = journal
        .entries
        .iter()
        .filter(|e| stash.iter().any(|row| row.commit == e.commit))
        .cloned()
        .collect();
    // hk-looking entries that appeared since the journal began and that the
    // journal does not name
    let unrecorded: Vec<String> = stash
        .iter()
        .filter(|row| {
            (message_pid(&row.subject) == Some(journal.pid)
                || (is_intent_to_add(&row.subject)
                    && !journal.stashes_before.contains(&row.commit)))
                && !journal.stashes_before.contains(&row.commit)
                && !journal.entries.iter().any(|e| e.commit == row.commit)
        })
        .map(|row| row.commit.clone())
        .collect();
    if !unrecorded.is_empty() {
        let mut commits: Vec<String> = present.iter().map(|e| e.commit.clone()).collect();
        commits.extend(unrecorded);
        return Action::Report {
            reason: ReportReason::UnrecordedEntry,
            commits,
        };
    }
    if present.is_empty() {
        return Action::Discard(if journal.entries.is_empty() {
            "hk had not stashed anything yet"
        } else {
            "its stash entries are already gone"
        });
    }
    let count = |kind| present.iter().filter(|e| e.kind == kind).count();
    if count(StashKind::Unstaged) > 1 || count(StashKind::IntentToAdd) > 1 {
        return Action::Report {
            reason: ReportReason::Ambiguous,
            commits: present.into_iter().map(|e| e.commit).collect(),
        };
    }
    if !worktree_clean {
        return Action::Report {
            reason: ReportReason::DirtyWorktree,
            commits: present.into_iter().map(|e| e.commit).collect(),
        };
    }
    Action::Restore(present)
}

/// The message for a journal hk does not act on. Names the exact command that
/// brings the changes back.
pub fn report_message(
    journal: &Journal,
    path: &Path,
    reason: &ReportReason,
    commits: &[String],
) -> String {
    let why = match reason {
        ReportReason::UnknownOwner(_) => String::new(),
        ReportReason::DirtyWorktree => {
            "The working tree has changes of its own, so hk did not apply them itself.".to_string()
        }
        ReportReason::UnrecordedEntry => {
            "hk cannot tell which stash entries are its own, so it did not apply them itself."
                .to_string()
        }
        ReportReason::Ambiguous => {
            "More than one stash entry is recorded, so hk did not apply them itself.".to_string()
        }
    };
    let commands = commits
        .iter()
        .map(|c| format!("  git stash apply {c}"))
        .collect::<Vec<_>>()
        .join("\n");
    if let ReportReason::UnknownOwner(why) = reason {
        return format!(
            "{} has your unstaged changes stashed, and hk cannot tell whether it is still running: {why}. hk restored and dropped nothing. If it is running, let it finish. If it is not, get the changes back with:\n{commands}\n\
             hk keeps reminding you until the stash entries are dropped (`git stash list`, then `git stash drop`), or until you delete {}.",
            journal.describe(),
            path.display()
        );
    }
    format!(
        "{} was stopped while it had your unstaged changes stashed, and they were not put back. {why}\n\
         To get them back, once the working tree is ready for them, run:\n{commands}\n\
         If git reports conflicts in files that are also staged, `git restore --source=<commit> --worktree -- <file>` takes the stashed version of one file.\n\
         hk keeps reminding you until the stash entries are dropped (`git stash list`, then `git stash drop`), or until you delete {}.",
        journal.describe(),
        path.display()
    )
}

/// Whether `git status --porcelain=v1 -z --no-renames` output lists a change
/// that is not staged, an untracked file or an unmerged path. Changes that are
/// only staged do not count: they are what hk's stash left in place.
pub fn status_has_worktree_changes(porcelain_z: &str) -> bool {
    porcelain_z
        .split('\0')
        .filter(|rec| rec.len() >= 3)
        .any(|rec| {
            let mut xy = rec.chars();
            let (x, y) = (xy.next().unwrap_or(' '), xy.next().unwrap_or(' '));
            x == '?'
                || x == 'U'
                || y == 'U'
                || (x == 'A' && y == 'A')
                || (x == 'D' && y == 'D')
                || y != ' '
        })
}

/// A value that differs between two processes that had the same pid.
pub fn process_start_token(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // The command name, in parentheses, may itself contain spaces
        let rest = stat.rsplit_once(')')?.1;
        let start = rest.split_whitespace().nth(19)?;
        let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        Some(format!("{boot}:{start}"))
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        let out = std::process::Command::new("ps")
            .args(["-o", "lstart=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let text = String::from_utf8(out.stdout).ok()?.trim().to_string();
        (out.status.success() && !text.is_empty()).then_some(text)
    }
    #[cfg(windows)]
    {
        windows::creation_time(pid).map(|t| t.to_string())
    }
}

/// Whether a process with this pid exists. Errs toward `true`: hk never
/// touches the journal of a process it cannot rule out.
pub fn process_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return true;
        };
        if pid <= 0 {
            return true;
        }
        // SAFETY: signal 0 only checks that the process can be signalled
        let rc = unsafe { libc::kill(pid, 0) };
        rc == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }
    #[cfg(windows)]
    {
        windows::is_alive(pid)
    }
}

#[cfg(windows)]
mod windows {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INVALID_PARAMETER, FILETIME, GetLastError, HANDLE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    const STILL_ACTIVE: u32 = 259;

    fn open(pid: u32) -> Result<HANDLE, u32> {
        // SAFETY: plain FFI call; the handle is closed by the callers
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            // SAFETY: reads the calling thread's last error
            Err(unsafe { GetLastError() })
        } else {
            Ok(handle)
        }
    }

    pub fn is_alive(pid: u32) -> bool {
        match open(pid) {
            Ok(handle) => {
                let mut code = 0u32;
                // SAFETY: `handle` is a valid process handle and `code` a valid out pointer
                let ok = unsafe { GetExitCodeProcess(handle, &mut code) };
                // SAFETY: closing the handle opened above
                unsafe { CloseHandle(handle) };
                ok == 0 || code == STILL_ACTIVE
            }
            Err(code) => code != ERROR_INVALID_PARAMETER,
        }
    }

    pub fn creation_time(pid: u32) -> Option<u64> {
        let handle = open(pid).ok()?;
        let zero = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
        // SAFETY: `handle` is valid and the out pointers point to live FILETIMEs
        let ok =
            unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) };
        // SAFETY: closing the handle opened above
        unsafe { CloseHandle(handle) };
        (ok != 0)
            .then(|| (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal() -> Journal {
        Journal {
            version: VERSION,
            pid: 4242,
            pid_start: Some("boot:100".into()),
            owner_host: Some("host|pid:[1]".into()),
            timestamp: 1_700_000_000,
            hook: "pre-commit".into(),
            worktree: "/repo".into(),
            stashes_before: vec!["old".into()],
            entries: vec![],
        }
    }

    fn row(commit: &str, subject: &str) -> StashRow {
        StashRow {
            commit: commit.into(),
            subject: subject.into(),
        }
    }

    fn entry(commit: &str) -> JournalEntry {
        JournalEntry {
            commit: commit.into(),
            kind: StashKind::Unstaged,
        }
    }

    #[test]
    fn roundtrips_through_json() {
        let mut j = journal();
        j.entries.push(JournalEntry {
            commit: "abc".into(),
            kind: StashKind::IntentToAdd,
        });
        let text = j.to_json().unwrap();
        assert!(text.contains("\"intent-to-add\""));
        assert_eq!(Journal::parse(&text).unwrap(), j);
    }

    #[test]
    fn rejects_garbage_and_unknown_versions() {
        assert!(Journal::parse("not json").is_err());
        let mut j = journal();
        j.version = 99;
        assert!(Journal::parse(&j.to_json().unwrap()).is_err());
    }

    const HERE: Option<&str> = Some("host|pid:[1]");

    #[test]
    fn owner_gone_when_same_place_and_dead_or_pid_reused() {
        let j = journal();
        assert_eq!(j.owner(HERE, false, None), Owner::Gone);
        assert_eq!(j.owner(HERE, true, Some("boot:100")), Owner::Running);
        assert_eq!(j.owner(HERE, true, Some("boot:999")), Owner::Gone);
        // Without start information a live pid is trusted
        assert_eq!(j.owner(HERE, true, None), Owner::Running);
        let mut no_start = journal();
        no_start.pid_start = None;
        assert_eq!(no_start.owner(HERE, true, Some("boot:999")), Owner::Running);
    }

    #[test]
    fn owner_is_unknown_from_another_host_or_namespace() {
        let j = journal();
        // The pid is not visible here, but that proves nothing
        for there in [Some("other|pid:[1]"), Some("host|pid:[2]"), None] {
            assert!(matches!(j.owner(there, false, None), Owner::Unknown(_)));
            // Even a visible pid is a different process in another namespace
            assert!(matches!(
                j.owner(there, true, Some("boot:999")),
                Owner::Unknown(_)
            ));
        }
    }

    #[test]
    fn journal_without_identity_is_unknown_unless_its_pid_is_visible() {
        let mut old = journal();
        old.owner_host = None;
        assert!(matches!(old.owner(HERE, false, None), Owner::Unknown(_)));
        // As before for a pid that is alive
        assert_eq!(old.owner(HERE, true, Some("boot:100")), Owner::Running);
        assert_eq!(old.owner(HERE, true, Some("boot:999")), Owner::Gone);
    }

    #[test]
    fn identity_is_optional_in_json_and_this_process_has_one() {
        let mut j = journal();
        j.owner_host = None;
        let text = j.to_json().unwrap();
        assert!(!text.contains("owner_host"));
        assert_eq!(Journal::parse(&text).unwrap().owner_host, None);
        assert_eq!(
            Journal::new("h", Path::new("/r"), vec![]).owner_host,
            host_identity()
        );
        assert!(host_identity().is_some());
    }

    #[cfg(unix)]
    #[test]
    fn read_only_journal_lock_file_can_still_be_locked() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let lock_path = dir.path().join(LOCK_NAME);
        std::fs::write(&lock_path, "").unwrap();
        std::fs::set_permissions(&lock_path, std::fs::Permissions::from_mode(0o444)).unwrap();
        let held = JournalLock::acquire(&path, None).unwrap().expect("locked");
        assert!(
            JournalLock::acquire_for(&path, std::time::Duration::from_millis(50), None)
                .unwrap()
                .is_none()
        );
        drop(held);
    }

    #[cfg(unix)]
    #[test]
    fn journal_lock_file_honors_the_shared_mode_over_the_umask() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let _held = JournalLock::acquire(&path, Some(0o660))
            .unwrap()
            .expect("locked");
        let mode = std::fs::metadata(dir.path().join(LOCK_NAME))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o660);
    }

    #[test]
    fn nothing_stashed_is_discarded() {
        let stash = [row("old", "On main: something")];
        assert!(matches!(
            decide(&journal(), &stash, true),
            Action::Discard(_)
        ));
    }

    #[test]
    fn recorded_entry_restores_when_clean() {
        let mut j = journal();
        j.entries.push(entry("new"));
        let stash = [row("new", "On main: hk: 4242-9-0"), row("old", "x")];
        assert_eq!(
            decide(&j, &stash, true),
            Action::Restore(vec![entry("new")])
        );
    }

    #[test]
    fn recorded_entry_is_reported_when_worktree_is_dirty() {
        let mut j = journal();
        j.entries.push(entry("new"));
        let stash = [row("new", "On main: hk: 4242-9-0")];
        assert_eq!(
            decide(&j, &stash, false),
            Action::Report {
                reason: ReportReason::DirtyWorktree,
                commits: vec!["new".into()]
            }
        );
    }

    #[test]
    fn already_restored_entry_is_discarded() {
        let mut j = journal();
        j.entries.push(entry("gone"));
        assert!(matches!(decide(&j, &[], true), Action::Discard(_)));
    }

    #[test]
    fn unrecorded_hk_entry_is_reported_never_restored() {
        // hk died between creating its entry and recording it
        let stash = [
            row("new", "On main: hk: 4242-1a2b-0"),
            // Another process's entry, and one that predates the journal
            row("other", "On main: hk: 777-1a2b-0"),
            row("old", "On main: hk: 4242-1a2b-1"),
        ];
        assert_eq!(
            decide(&journal(), &stash, true),
            Action::Report {
                reason: ReportReason::UnrecordedEntry,
                commits: vec!["new".into()]
            }
        );
        // Also next to a recorded one
        let mut j = journal();
        j.entries.push(entry("a"));
        let stash = [
            row("a", "On main: hk: 4242-9-0"),
            row("b", "On main: hk: intent-to-add files"),
        ];
        assert_eq!(
            decide(&j, &stash, true),
            Action::Report {
                reason: ReportReason::UnrecordedEntry,
                commits: vec!["a".into(), "b".into()]
            }
        );
    }

    #[test]
    fn empty_journal_is_never_discarded_while_its_pid_has_an_entry() {
        // libgit2 prefixes the message with `On <branch>: `, git with `On <branch>: ` too
        for subject in [
            "On main: hk: 4242-1a2b-0",
            "On feature/x: hk: 4242-1a2b-3 (intent-to-add files)",
            "WIP on main: hk: 4242-ff-1",
        ] {
            let j = journal();
            assert!(j.entries.is_empty());
            assert_eq!(
                decide(&j, &[row("new", subject)], true),
                Action::Report {
                    reason: ReportReason::UnrecordedEntry,
                    commits: vec!["new".into()]
                },
                "{subject}"
            );
        }
        // Nothing of this pid's is there: safe to discard
        let stash = [row("x", "On main: hk: 99-1-0"), row("y", "On main: wip")];
        assert!(matches!(
            decide(&journal(), &stash, true),
            Action::Discard(_)
        ));
    }

    #[test]
    fn message_pid_reads_hks_per_run_message() {
        assert_eq!(
            message_pid("On main: hk: 4242-1f-3 (intent-to-add files)"),
            Some(4242)
        );
        assert_eq!(message_pid("On main: hk: 4242-1f-3"), Some(4242));
        assert_eq!(message_pid("hk: 4242-1f-3"), Some(4242));
        assert_eq!(message_pid("WIP on main: hk: 7-1f-3"), Some(7));
        assert_eq!(message_pid("On main: hk"), None);
        assert_eq!(message_pid("On main: fix hk: 12-1-1"), None);
        assert!(is_intent_to_add("On main: hk: intent-to-add files"));
        assert!(!is_intent_to_add("On main: hk: 4242-1f-3"));
    }

    #[test]
    fn report_names_the_exact_command() {
        let j = journal();
        let msg = report_message(
            &j,
            Path::new("/repo/.git/hk-pending-stash"),
            &ReportReason::DirtyWorktree,
            &["abc123".into()],
        );
        assert!(msg.contains("git stash apply abc123"));
        assert!(msg.contains("pid 4242"));
        assert!(msg.contains("/repo/.git/hk-pending-stash"));
    }

    #[test]
    fn porcelain_status_classification() {
        assert!(!status_has_worktree_changes(""));
        // staged only
        assert!(!status_has_worktree_changes(
            "M  a.txt\0A  b.txt\0D  c.txt\0"
        ));
        // unstaged edit, untracked file, unmerged
        assert!(status_has_worktree_changes("MM a.txt\0"));
        assert!(status_has_worktree_changes(" M a.txt\0"));
        assert!(status_has_worktree_changes("?? new.txt\0"));
        assert!(status_has_worktree_changes("UU a.txt\0"));
    }

    fn lock(path: &Path) -> JournalLock {
        JournalLock::acquire(path, None)
            .unwrap()
            .expect("lock is free")
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn begin_never_overwrites_an_existing_journal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut mine = OwnedJournal::begin(path.clone(), journal(), None)
            .unwrap()
            .expect("first journal is written");
        mine.record("abc", StashKind::Unstaged).unwrap();
        assert!(mine.has_entries());
        // A second writer is refused and the first one's content is intact
        assert!(
            OwnedJournal::begin(path.clone(), journal(), None)
                .unwrap()
                .is_none()
        );
        let on_disk = read(&path).unwrap().unwrap();
        assert_eq!(on_disk.entries, vec![entry("abc")]);
        mine.forget("abc").unwrap();
        assert!(read(&path).unwrap().unwrap().entries.is_empty());
        mine.remove();
        assert!(read(&path).unwrap().is_none());
        // Only the lock file is left: no temporary or claim files
        assert_eq!(names(dir.path()), vec![LOCK_NAME.to_string()]);
    }

    #[test]
    fn create_never_clobbers_even_an_unreadable_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let l = lock(&path);
        let first = journal();
        assert!(create(&path, &first, &l).unwrap());
        let mut second = journal();
        second.pid = 7;
        assert!(!create(&path, &second, &l).unwrap());
        assert_eq!(read(&path).unwrap().unwrap(), first);
        std::fs::write(&path, "garbage").unwrap();
        assert!(!create(&path, &second, &l).unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "garbage");
    }

    #[test]
    fn lock_is_exclusive_and_released_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let held = lock(&path);
        // A second hold, from another open file, waits and then gives up
        let started = std::time::Instant::now();
        assert!(
            JournalLock::acquire_for(&path, std::time::Duration::from_millis(100), None)
                .unwrap()
                .is_none()
        );
        assert!(started.elapsed() >= std::time::Duration::from_millis(100));
        drop(held);
        assert!(
            JournalLock::acquire_for(&path, std::time::Duration::from_millis(100), None)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn stale_temp_file_sharing_the_journals_inode_is_never_truncated() {
        // A process died after linking its temp file to the journal; a later
        // process with the same pid must not write through that name
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let live = journal();
        let l = lock(&path);
        assert!(create(&path, &live, &l).unwrap());
        let pid = std::process::id();
        let stale = dir.path().join(format!("{FILE_NAME}.{pid}.tmp"));
        std::fs::hard_link(&path, &stale).unwrap();
        // Writing temporaries (any number) never opens the stale name
        let mut next = journal();
        next.pid = 5;
        for _ in 0..3 {
            let tmp = write_temp(&path, &next, &l).unwrap();
            assert_ne!(tmp, stale);
            std::fs::remove_file(tmp).unwrap();
        }
        assert_eq!(read(&path).unwrap().unwrap(), live);
        assert_eq!(read(&stale).unwrap().unwrap(), live);
        // Replacing goes through a fresh inode, so the stale name keeps its content
        replace(&path, &next, &l).unwrap();
        assert_eq!(read(&path).unwrap().unwrap(), next);
        assert_eq!(read(&stale).unwrap().unwrap(), live);
        // Under the lock, leftovers are swept and the journal is untouched
        sweep_temp_files(&path, &l);
        assert_eq!(
            names(dir.path()),
            vec![FILE_NAME.to_string()]
                .into_iter()
                .chain([LOCK_NAME.to_string()])
                .collect::<Vec<_>>()
        );
        assert_eq!(read(&path).unwrap().unwrap(), next);
    }

    #[test]
    fn temp_names_do_not_repeat() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let l = lock(&path);
        let a = write_temp(&path, &journal(), &l).unwrap();
        let b = write_temp(&path, &journal(), &l).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn interrupted_discard_or_replace_leaves_the_journal_in_place() {
        // A crash after the temp file was written but before the rename:
        // the journal is still the old one and the leftover is swept later
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let l = lock(&path);
        let old = journal();
        assert!(create(&path, &old, &l).unwrap());
        let mut newer = journal();
        newer.pid = 11;
        let leftover = write_temp(&path, &newer, &l).unwrap();
        assert!(leftover.exists());
        assert_eq!(read(&path).unwrap().unwrap(), old);
        sweep_temp_files(&path, &l);
        assert!(!leftover.exists());
        assert_eq!(read(&path).unwrap().unwrap(), old);
    }

    #[test]
    fn remove_leaves_a_journal_another_process_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mine = OwnedJournal::begin(path.clone(), journal(), None)
            .unwrap()
            .unwrap();
        let mut other = journal();
        other.pid = 99;
        replace(&path, &other, &lock(&path)).unwrap();
        mine.remove();
        assert_eq!(read(&path).unwrap().unwrap(), other);
    }

    #[test]
    fn concurrent_creates_write_exactly_one_journal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let winners: usize = std::thread::scope(|s| {
            let handles: Vec<_> = (0..8u32)
                .map(|i| {
                    let path = path.clone();
                    s.spawn(move || {
                        let mut j = journal();
                        j.pid = 1000 + i;
                        OwnedJournal::begin(path, j, None).unwrap().is_some()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| usize::from(h.join().unwrap()))
                .sum()
        });
        assert_eq!(winners, 1);
        assert!(read(&path).unwrap().is_some());
        assert_eq!(
            names(dir.path()),
            vec![FILE_NAME.to_string(), LOCK_NAME.to_string()]
        );
    }

    #[test]
    fn concurrent_record_recover_and_remove_never_corrupt_the_journal() {
        // The owner records and forgets entries while other threads take the
        // lock the way a recovering run does (read, decide, maybe discard a
        // journal that is not theirs to discard). Every read sees a whole
        // journal, and discarding is only done by the one that owns it.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut mine = OwnedJournal::begin(path.clone(), journal(), None)
            .unwrap()
            .unwrap();
        let stop = std::sync::atomic::AtomicBool::new(false);
        std::thread::scope(|s| {
            for _ in 0..3 {
                s.spawn(|| {
                    while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                        let l = lock(&path);
                        sweep_temp_files(&path, &l);
                        // Whatever is there is complete and parseable
                        let j = read(&path).unwrap().expect("journal exists");
                        assert_eq!(j.pid, 4242);
                        drop(l);
                    }
                });
            }
            for i in 0..50 {
                let commit = format!("c{i}");
                mine.record(&commit, StashKind::Unstaged).unwrap();
                mine.forget(&commit).unwrap();
            }
            stop.store(true, std::sync::atomic::Ordering::SeqCst);
        });
        mine.remove();
        assert_eq!(names(dir.path()), vec![LOCK_NAME.to_string()]);
    }

    #[test]
    fn this_process_is_alive_and_has_a_stable_start_token() {
        let pid = std::process::id();
        assert!(process_is_alive(pid));
        assert_eq!(process_start_token(pid), process_start_token(pid));
    }

    #[cfg(unix)]
    #[test]
    fn exited_process_is_not_alive() {
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        assert!(!process_is_alive(pid));
    }
}
