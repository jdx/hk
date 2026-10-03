//! Killing the processes a cancelled step left behind (Windows).
//!
//! When a step is cancelled, ensembler terminates only the process it spawned.
//! On Windows that is usually `cmd.exe` (a shell string, or a `.cmd` shim), and
//! terminating it does not touch its children: the tool it started keeps
//! running, and keeps writing, after hk has released the file locks and
//! restored the stash. So hk snapshots the processes below its own while the
//! shell is still alive, lets ensembler cancel, and then ends the rest.

use tokio_util::sync::CancellationToken;

/// Image names of hk's own helpers when they are direct children of hk. A
/// step's `git` is a child of the step's shell, or of a tool, never of hk
/// itself, so only these are spared: killing hk's own `git` halfway through an
/// index update leaves a stale `index.lock` behind.
#[cfg_attr(not(windows), allow(dead_code))]
const HELPERS: &[&str] = &["git.exe"];

/// How many times to look again for processes started while the first ones
/// were being ended.
#[cfg_attr(not(windows), allow(dead_code))]
const RESCANS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Proc {
    pub pid: u32,
    pub ppid: u32,
    /// Creation time (any monotonic unit). A process whose creation time could
    /// not be read has 0 and is never treated as a child or ended, because
    /// without it a reused process number cannot be told from the original.
    pub created: u64,
    pub exe: String,
}

/// Add to `found` the processes at any depth below `parents`, parents before
/// children. Each parent is a `(pid, creation time)` pair.
///
/// Windows does not reparent orphans, so `ppid` can name a dead process whose
/// number another process has since reused. A real child is never older than
/// its parent, and a parent whose number is now held by a process with another
/// creation time is not the parent any more; both rule those out. The direct
/// children of `parents` that are named in `skip_direct` are left out, with
/// their trees.
#[cfg_attr(not(windows), allow(dead_code))]
fn collect(procs: &[Proc], parents: &[(u32, u64)], skip_direct: &[&str], found: &mut Vec<Proc>) {
    let mut level = parents.to_vec();
    let mut direct = true;
    while !level.is_empty() {
        let mut next = Vec::new();
        for (parent, parent_created) in level {
            let reused = procs
                .iter()
                .any(|p| p.pid == parent && p.created != parent_created);
            if parent_created == 0 || reused {
                continue;
            }
            for p in procs {
                let is_child = p.ppid == parent
                    && p.pid != parent
                    && p.created != 0
                    && p.created >= parent_created;
                let skipped = direct && skip_direct.iter().any(|s| p.exe.eq_ignore_ascii_case(s));
                if is_child && !skipped && !found.iter().any(|f| f.pid == p.pid) {
                    found.push(p.clone());
                    next.push((p.pid, p.created));
                }
            }
        }
        level = next;
        direct = false;
    }
}

/// The processes at any depth below `root`, apart from the trees of its direct
/// children named in `skip_direct`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn descendants(procs: &[Proc], root: u32, skip_direct: &[&str]) -> Vec<Proc> {
    let root_created = procs
        .iter()
        .find(|p| p.pid == root)
        .map_or(0, |p| p.created);
    let mut found = Vec::new();
    collect(procs, &[(root, root_created)], skip_direct, &mut found);
    found
}

/// The processes below `known` that are not in it yet: what they started after
/// `known` was listed.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn newcomers(procs: &[Proc], known: &[Proc]) -> Vec<Proc> {
    let parents: Vec<(u32, u64)> = known.iter().map(|p| (p.pid, p.created)).collect();
    let mut found = known.to_vec();
    collect(procs, &parents, &[], &mut found);
    found.split_off(known.len())
}

/// Cancel `failed`, and on Windows also end everything the running steps'
/// commands started, before returning.
///
/// The processes are listed first, while their shells are alive: once
/// ensembler has terminated a `cmd.exe`, its children no longer lead back to
/// hk. Elsewhere ensembler signals the command's process group, so this only
/// cancels.
pub(crate) async fn cancel_running_steps(failed: &CancellationToken) {
    #[cfg(windows)]
    {
        let victims = tokio::task::spawn_blocking(windows::snapshot)
            .await
            .unwrap_or_default();
        failed.cancel();
        debug!(
            "cancelled: ending {} process(es) below hk: {:?}",
            victims.len(),
            victims
        );
        let _ = tokio::task::spawn_blocking(move || windows::kill(victims)).await;
    }
    #[cfg(not(windows))]
    failed.cancel();
}

#[cfg(windows)]
mod windows {
    use super::{HELPERS, Proc, RESCANS, descendants, newcomers};
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
    };

    /// The creation time of an open process handle, or 0.
    ///
    /// # Safety
    /// `handle` must be a valid process handle with query access.
    unsafe fn handle_creation_time(handle: windows_sys::Win32::Foundation::HANDLE) -> u64 {
        let zero = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
        // SAFETY: the caller guarantees the handle; the out pointers are locals.
        let ok =
            unsafe { GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) };
        if ok == 0 {
            return 0;
        }
        (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime)
    }

    fn creation_time(pid: u32) -> u64 {
        // SAFETY: plain Win32 calls; the handle is closed before returning.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return 0;
            }
            let created = handle_creation_time(handle);
            CloseHandle(handle);
            created
        }
    }

    fn processes() -> Vec<Proc> {
        let mut procs = Vec::new();
        // SAFETY: the snapshot handle is closed, and the entry is initialised
        // with its size as the API requires.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return procs;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snapshot, &mut entry);
            while more != 0 {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                procs.push(Proc {
                    pid: entry.th32ProcessID,
                    ppid: entry.th32ParentProcessID,
                    created: 0,
                    exe: String::from_utf16_lossy(&entry.szExeFile[..len]),
                });
                more = Process32NextW(snapshot, &mut entry);
            }
            CloseHandle(snapshot);
        }
        for p in &mut procs {
            p.created = creation_time(p.pid);
        }
        procs
    }

    /// The processes below hk's own, apart from hk's helpers.
    pub(super) fn snapshot() -> Vec<Proc> {
        descendants(&processes(), std::process::id(), HELPERS)
    }

    /// End one process, if it is still the one that was listed, and wait,
    /// briefly, for it to be gone.
    fn terminate(victim: &Proc) {
        // A victim without a creation time is never ended: its number may
        // belong to an unrelated process by now.
        if victim.created == 0 {
            return;
        }
        // SAFETY: plain Win32 calls; the handle is closed after use.
        unsafe {
            let handle = OpenProcess(
                PROCESS_TERMINATE | PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                victim.pid,
            );
            if handle.is_null() {
                return;
            }
            if handle_creation_time(handle) == victim.created {
                let ended = TerminateProcess(handle, 1);
                let waited = WaitForSingleObject(handle, 2000);
                debug!(
                    "terminated {} ({}): ended={ended} waited={waited}",
                    victim.pid, victim.exe
                );
            }
            CloseHandle(handle);
        }
    }

    /// End the processes and whatever they started in the meantime: a tool can
    /// start a child after the list was taken and before its shell was ended.
    pub(super) fn kill(victims: Vec<Proc>) {
        let mut known = victims.clone();
        let mut pending = victims;
        for _ in 0..=RESCANS {
            for victim in &pending {
                terminate(victim);
            }
            pending = newcomers(&processes(), &known);
            if pending.is_empty() {
                return;
            }
            known.extend(pending.iter().cloned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, ppid: u32, created: u64, exe: &str) -> Proc {
        Proc {
            pid,
            ppid,
            created,
            exe: exe.to_string(),
        }
    }

    fn pids(procs: &[Proc]) -> Vec<u32> {
        procs.iter().map(|p| p.pid).collect()
    }

    #[test]
    fn finds_grandchildren_of_the_root() {
        let procs = [
            p(1, 0, 10, "hk.exe"),
            p(2, 1, 20, "cmd.exe"),
            p(3, 2, 30, "node.exe"),
            p(4, 3, 40, "node.exe"),
            p(5, 99, 50, "other.exe"),
        ];
        assert_eq!(pids(&descendants(&procs, 1, &[])), vec![2, 3, 4]);
    }

    #[test]
    fn does_not_reach_orphans_of_a_dead_parent_from_the_root() {
        // cmd.exe (2) is gone: node (3) still names it, so it is unreachable
        // from hk, which is why the snapshot is taken while cmd.exe lives.
        let procs = [p(1, 0, 10, "hk.exe"), p(3, 2, 30, "node.exe")];
        assert!(descendants(&procs, 1, &[]).is_empty());
    }

    #[test]
    fn ignores_a_parent_number_reused_by_a_newer_process() {
        // pid 2 was reused by a process created after its supposed child.
        let procs = [
            p(1, 0, 10, "hk.exe"),
            p(2, 1, 50, "new.exe"),
            p(3, 2, 30, "old-orphan.exe"),
        ];
        assert_eq!(pids(&descendants(&procs, 1, &[])), vec![2]);
    }

    #[test]
    fn leaves_hks_own_helpers_alone() {
        let procs = [
            p(1, 0, 10, "hk.exe"),
            p(2, 1, 20, "git.exe"),
            p(3, 2, 30, "git-remote-https.exe"),
            p(4, 1, 20, "cmd.exe"),
        ];
        assert_eq!(pids(&descendants(&procs, 1, HELPERS)), vec![4]);
    }

    #[test]
    fn ends_a_git_started_by_a_step() {
        // Only hk's direct children are helpers; a step's git runs under its shell.
        let procs = [
            p(1, 0, 10, "hk.exe"),
            p(2, 1, 20, "cmd.exe"),
            p(3, 2, 30, "git.exe"),
            p(4, 3, 40, "git-remote-https.exe"),
        ];
        assert_eq!(pids(&descendants(&procs, 1, HELPERS)), vec![2, 3, 4]);
    }

    #[test]
    fn never_trusts_a_process_with_an_unknown_creation_time() {
        let procs = [
            p(1, 0, 10, "hk.exe"),
            p(2, 1, 0, "unknown.exe"),
            p(3, 1, 20, "cmd.exe"),
            p(4, 3, 0, "unknown-child.exe"),
        ];
        assert_eq!(pids(&descendants(&procs, 1, &[])), vec![3]);
    }

    #[test]
    fn terminates_on_a_parent_cycle() {
        let procs = [p(1, 2, 10, "a.exe"), p(2, 1, 20, "b.exe")];
        assert_eq!(pids(&descendants(&procs, 1, &[])), vec![2]);
    }

    #[test]
    fn finds_what_a_listed_process_started_later() {
        // cmd.exe (2) and node (3) were listed; node then started a child (5)
        // and cmd.exe has been ended since, so 5 hangs below 3.
        let known = [p(2, 1, 20, "cmd.exe"), p(3, 2, 30, "node.exe")];
        let procs = [
            p(3, 2, 30, "node.exe"),
            p(5, 3, 50, "child.exe"),
            p(6, 5, 60, "grandchild.exe"),
            p(7, 99, 70, "unrelated.exe"),
        ];
        assert_eq!(pids(&newcomers(&procs, &known)), vec![5, 6]);
    }

    #[test]
    fn newcomers_of_a_dead_parent_are_found_by_its_number() {
        // The listed node (3) has exited; its orphan (5) still names it.
        let known = [p(3, 2, 30, "node.exe")];
        let procs = [p(5, 3, 50, "orphan.exe")];
        assert_eq!(pids(&newcomers(&procs, &known)), vec![5]);
    }

    #[test]
    fn newcomers_skip_a_parent_number_reused_since() {
        let known = [p(3, 2, 30, "node.exe")];
        // pid 3 now belongs to an unrelated, newer process and its child.
        let procs = [p(3, 1, 90, "unrelated.exe"), p(8, 3, 95, "its-child.exe")];
        assert!(newcomers(&procs, &known).is_empty());
    }
}
