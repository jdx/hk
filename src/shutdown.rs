//! Stopping a run on SIGINT, SIGTERM or SIGHUP (Ctrl+C, Ctrl+Break or a
//! closing console on Windows).
//!
//! The first signal cancels the run's token, which stops the running steps'
//! commands and lets the hook put the stashed changes back and delete its
//! journal before hk exits with the conventional status, 128 plus the signal
//! number. Restoring has [`GRACE`] to finish; after that hk exits and leaves
//! the journal for the next run to recover from. The same signal twice exits
//! at once.
//!
//! Restoring never depends on the terminal: when the terminal is gone
//! (SIGHUP, a closed console) hk's output is redirected to the null device,
//! because writing to a vanished terminal fails with EIO and a failed print
//! panics.

use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

/// How long hk gets to stop its steps and restore the stash after a signal.
const GRACE: Duration = Duration::from_secs(10);

/// The exit status of the first termination signal received, 0 if none.
static EXIT_CODE: AtomicI32 = AtomicI32::new(0);

/// The status hk exits with after being terminated by a signal.
pub fn exit_code() -> Option<i32> {
    match EXIT_CODE.load(Ordering::SeqCst) {
        0 => None,
        code => Some(code),
    }
}

/// Cancels `cancel` on the first termination signal.
pub fn watch(cancel: CancellationToken) {
    let Some(mut signals) = Signals::new() else {
        return;
    };
    tokio::spawn(async move {
        let mut first: Option<i32> = None;
        loop {
            let (code, terminal_gone) = signals.next().await;
            if terminal_gone {
                detach_terminal();
            }
            match first {
                None => {
                    first = Some(code);
                    EXIT_CODE.store(code, Ordering::SeqCst);
                    cancel.cancel();
                    tokio::spawn(async move {
                        tokio::time::sleep(GRACE).await;
                        std::process::exit(code);
                    });
                }
                // The same signal again: stop waiting
                Some(prev) if prev == code => std::process::exit(code),
                Some(_) => {}
            }
        }
    });
}

#[cfg(unix)]
struct Signals {
    int: tokio::signal::unix::Signal,
    term: tokio::signal::unix::Signal,
    hup: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl Signals {
    fn new() -> Option<Self> {
        use tokio::signal::unix::{SignalKind, signal};
        let build = || -> std::io::Result<Self> {
            Ok(Self {
                int: signal(SignalKind::interrupt())?,
                term: signal(SignalKind::terminate())?,
                hup: signal(SignalKind::hangup())?,
            })
        };
        build()
            .inspect_err(|err| log::warn!("Failed to watch for termination signals: {err}"))
            .ok()
    }

    /// The exit status for the next signal, and whether it means the terminal is gone.
    async fn next(&mut self) -> (i32, bool) {
        tokio::select! {
            _ = self.int.recv() => (128 + libc::SIGINT, false),
            _ = self.term.recv() => (128 + libc::SIGTERM, false),
            _ = self.hup.recv() => (128 + libc::SIGHUP, true),
        }
    }
}

/// Points stdout and stderr at /dev/null, so nothing hk prints from here on can fail.
#[cfg(unix)]
fn detach_terminal() {
    use std::os::fd::AsRawFd;
    if let Ok(null) = std::fs::OpenOptions::new().write(true).open("/dev/null") {
        for fd in [libc::STDOUT_FILENO, libc::STDERR_FILENO] {
            // SAFETY: duplicating an open descriptor onto stdout/stderr
            unsafe { libc::dup2(null.as_raw_fd(), fd) };
        }
    }
}

#[cfg(windows)]
struct Signals {
    c: tokio::signal::windows::CtrlC,
    brk: tokio::signal::windows::CtrlBreak,
    close: tokio::signal::windows::CtrlClose,
    logoff: tokio::signal::windows::CtrlLogoff,
    shutdown: tokio::signal::windows::CtrlShutdown,
}

#[cfg(windows)]
impl Signals {
    fn new() -> Option<Self> {
        use tokio::signal::windows;
        let build = || -> std::io::Result<Self> {
            Ok(Self {
                c: windows::ctrl_c()?,
                brk: windows::ctrl_break()?,
                close: windows::ctrl_close()?,
                logoff: windows::ctrl_logoff()?,
                shutdown: windows::ctrl_shutdown()?,
            })
        };
        build()
            .inspect_err(|err| log::warn!("Failed to watch for console events: {err}"))
            .ok()
    }

    /// Statuses follow the POSIX numbering: Ctrl+C is SIGINT (130), Ctrl+Break
    /// is SIGQUIT (131), and a closing console, logoff or shutdown is SIGHUP (129).
    /// Windows ends the process a few seconds after those three, so restoring
    /// has to be quick.
    async fn next(&mut self) -> (i32, bool) {
        tokio::select! {
            _ = self.c.recv() => (130, false),
            _ = self.brk.recv() => (131, false),
            _ = self.close.recv() => (129, true),
            _ = self.logoff.recv() => (129, true),
            _ = self.shutdown.recv() => (129, true),
        }
    }
}

/// Points stdout and stderr at NUL, so nothing hk prints from here on can fail.
#[cfg(windows)]
fn detach_terminal() {
    use std::os::windows::io::IntoRawHandle;
    use windows_sys::Win32::System::Console::{STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle};
    for std_handle in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        if let Ok(nul) = std::fs::OpenOptions::new().write(true).open("NUL") {
            // SAFETY: installs a handle that stays open for the rest of the process
            unsafe { SetStdHandle(std_handle, nul.into_raw_handle() as _) };
        }
    }
}
