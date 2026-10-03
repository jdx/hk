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

    /// Whether the process that wrote this journal is gone. `alive` and
    /// `current_start` describe the process that now has `self.pid`.
    pub fn owner_is_gone(&self, alive: bool, current_start: Option<&str>) -> bool {
        if !alive {
            return true;
        }
        // Alive, but a different process than the one that wrote the journal
        matches!((self.pid_start.as_deref(), current_start), (Some(a), Some(b)) if a != b)
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

/// Reads the journal at `path`. `Ok(None)` when there is none.
pub fn read(path: &Path) -> Result<Option<Journal>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(Journal::parse(&text)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).wrap_err_with(|| format!("failed to read {}", path.display())),
    }
}

fn write_temp(path: &Path, journal: &Journal) -> Result<PathBuf> {
    let tmp = path.with_file_name(format!("{FILE_NAME}.{}.tmp", std::process::id()));
    std::fs::write(&tmp, journal.to_json()?)
        .wrap_err_with(|| format!("failed to write {}", tmp.display()))?;
    Ok(tmp)
}

/// Writes `journal` at `path` atomically and only if nothing is there:
/// `Ok(false)` leaves the file that is. The content is written to a temporary
/// file first, so a reader never sees a partial journal.
fn create(path: &Path, journal: &Journal) -> Result<bool> {
    let tmp = write_temp(path, journal)?;
    let result = match std::fs::hard_link(&tmp, path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        // A filesystem without hard links: the check-then-rename below is
        // atomic apart from a concurrent hk creating its own journal
        Err(_) if !path.exists() => std::fs::rename(&tmp, path)
            .map(|()| true)
            .wrap_err_with(|| format!("failed to write {}", path.display())),
        Err(_) => Ok(false),
    };
    let _ = std::fs::remove_file(&tmp);
    result
}

/// Replaces the journal at `path` atomically.
fn replace(path: &Path, journal: &Journal) -> Result<()> {
    let tmp = write_temp(path, journal)?;
    std::fs::rename(&tmp, path).wrap_err_with(|| {
        let _ = std::fs::remove_file(&tmp);
        format!("failed to write {}", path.display())
    })
}

/// The journal this process owns while it has changes stashed.
#[derive(Debug)]
pub struct OwnedJournal {
    path: PathBuf,
    journal: Journal,
}

impl OwnedJournal {
    /// Writes a journal with no entries yet, before the worktree is touched.
    /// `Ok(None)` when a journal already exists, whose owner or leftovers hk
    /// must not overwrite.
    pub fn begin(path: PathBuf, journal: Journal) -> Result<Option<Self>> {
        Ok(create(&path, &journal)?.then_some(Self { path, journal }))
    }

    pub fn has_entries(&self) -> bool {
        !self.journal.entries.is_empty()
    }

    /// Records a stash entry hk created.
    pub fn record(&mut self, commit: &str, kind: StashKind) -> Result<()> {
        if self.journal.entries.iter().any(|e| e.commit == commit) {
            return Ok(());
        }
        self.journal.entries.push(JournalEntry {
            commit: commit.to_string(),
            kind,
        });
        replace(&self.path, &self.journal)
    }

    /// Forgets an entry hk dropped itself.
    pub fn forget(&mut self, commit: &str) -> Result<()> {
        self.journal.entries.retain(|e| e.commit != commit);
        replace(&self.path, &self.journal)
    }

    /// Deletes the journal: the changes are back in the worktree.
    pub fn remove(self) {
        remove_file(&self.path);
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

/// Whether a stash subject is the intent-to-add entry's, which has no pid.
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
        ReportReason::DirtyWorktree => {
            "The working tree has changes of its own, so hk did not apply them itself."
        }
        ReportReason::UnrecordedEntry => {
            "hk cannot tell which stash entries are its own, so it did not apply them itself."
        }
        ReportReason::Ambiguous => {
            "More than one stash entry is recorded, so hk did not apply them itself."
        }
    };
    let commands = commits
        .iter()
        .map(|c| format!("  git stash apply {c}"))
        .collect::<Vec<_>>()
        .join("\n");
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

    #[test]
    fn owner_gone_when_dead_or_pid_reused() {
        let j = journal();
        assert!(j.owner_is_gone(false, None));
        assert!(!j.owner_is_gone(true, Some("boot:100")));
        assert!(j.owner_is_gone(true, Some("boot:999")));
        // Without start information a live pid is trusted
        assert!(!j.owner_is_gone(true, None));
        let mut unknown = journal();
        unknown.pid_start = None;
        assert!(!unknown.owner_is_gone(true, Some("boot:999")));
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
    fn message_pid_reads_hks_per_run_message() {
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

    #[test]
    fn begin_never_overwrites_an_existing_journal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut mine = OwnedJournal::begin(path.clone(), journal())
            .unwrap()
            .expect("first journal is written");
        mine.record("abc", StashKind::Unstaged).unwrap();
        assert!(mine.has_entries());
        // A second writer is refused and the first one's content is intact
        assert!(
            OwnedJournal::begin(path.clone(), journal())
                .unwrap()
                .is_none()
        );
        let on_disk = read(&path).unwrap().unwrap();
        assert_eq!(on_disk.entries, vec![entry("abc")]);
        mine.forget("abc").unwrap();
        assert!(read(&path).unwrap().unwrap().entries.is_empty());
        mine.remove();
        assert!(read(&path).unwrap().is_none());
        // No temporary files are left behind
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
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
