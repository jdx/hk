//! Killing the processes a cancelled step left behind (Windows).
//!
//! When a step is cancelled, ensembler terminates only the process it spawned.
//! On Windows that is usually `cmd.exe` (a shell string, or a `.cmd` shim), and
//! terminating it does not touch its children: the tool it started keeps
//! running, and keeps writing, after hk has released the file locks and
//! restored the stash. So hk snapshots the processes below its own while the
//! shell is still alive, lets ensembler cancel, and then ends the rest.

use tokio_util::sync::CancellationToken;

/// Image names of hk's own helpers. They are children of hk but never of a
/// step, and killing `git` halfway through an index update leaves a stale
/// `index.lock` behind, so their trees are left alone.
const HELPERS: &[&str] = &["git.exe"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Proc {
    pub pid: u32,
    pub ppid: u32,
    /// Creation time (any monotonic unit), or 0 when unknown.
    pub created: u64,
    pub exe: String,
}

/// The processes at any depth below `root`, ordered parents before children.
///
/// Windows does not reparent orphans, so `ppid` can name a dead process whose
/// number another process has since reused. A real child is never older than
/// its parent, which rules those out. Trees under `skip` image names are left
/// out.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn descendants(procs: &[Proc], root: u32, skip: &[&str]) -> Vec<Proc> {
    let created_of = |pid: u32| procs.iter().find(|p| p.pid == pid).map_or(0, |p| p.created);
    let mut found: Vec<Proc> = Vec::new();
    let mut parents = vec![(root, created_of(root))];
    while !parents.is_empty() {
        let mut next = Vec::new();
        for (parent, parent_created) in parents {
            for p in procs {
                let is_child = p.ppid == parent
                    && p.pid != parent
                    && (p.created == 0 || parent_created == 0 || p.created >= parent_created);
                let skipped = skip.iter().any(|s| p.exe.eq_ignore_ascii_case(s));
                if is_child && !skipped && !found.iter().any(|f| f.pid == p.pid) && p.pid != root {
                    found.push(p.clone());
                    next.push((p.pid, p.created));
                }
            }
        }
        parents = next;
    }
    found
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
        let _ = tokio::task::spawn_blocking(move || windows::kill(&victims)).await;
    }
    #[cfg(not(windows))]
    failed.cancel();
}

#[cfg(windows)]
mod windows {
    use super::{HELPERS, Proc, descendants};
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
        PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
    };

    fn creation_time(pid: u32) -> u64 {
        // SAFETY: plain Win32 calls; the handle is closed before returning.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return 0;
            }
            let zero = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };
            let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
            let ok = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user);
            CloseHandle(handle);
            if ok == 0 {
                return 0;
            }
            (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime)
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

    /// End each process and wait, briefly, for it to be gone.
    pub(super) fn kill(victims: &[Proc]) {
        for victim in victims {
            // SAFETY: plain Win32 calls; the handle is closed after use.
            unsafe {
                let handle = OpenProcess(
                    PROCESS_TERMINATE | PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                    0,
                    victim.pid,
                );
                if handle.is_null() {
                    continue;
                }
                // The number may belong to another process by now.
                let mut proc_created = 0;
                let zero = FILETIME {
                    dwLowDateTime: 0,
                    dwHighDateTime: 0,
                };
                let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
                if GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) != 0 {
                    proc_created = (u64::from(created.dwHighDateTime) << 32)
                        | u64::from(created.dwLowDateTime);
                }
                if victim.created == 0 || proc_created == victim.created {
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
    fn follows_orphans_of_a_dead_parent_only_through_live_links() {
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
    fn leaves_helper_trees_alone() {
        let procs = [
            p(1, 0, 10, "hk.exe"),
            p(2, 1, 20, "git.exe"),
            p(3, 2, 30, "git-remote-https.exe"),
            p(4, 1, 20, "cmd.exe"),
        ];
        assert_eq!(pids(&descendants(&procs, 1, HELPERS)), vec![4]);
    }

    #[test]
    fn terminates_on_a_parent_cycle() {
        let procs = [p(1, 2, 0, "a.exe"), p(2, 1, 0, "b.exe")];
        assert_eq!(pids(&descendants(&procs, 1, &[])), vec![2]);
    }
}
