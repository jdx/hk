pub use std::env::*;
use std::{path::PathBuf, sync::LazyLock};

use indexmap::IndexSet;

use crate::git::StashMethod;

// pub static HK_BIN: LazyLock<PathBuf> =
//     LazyLock::new(|| current_exe().unwrap().canonicalize().unwrap());
// pub static CWD: LazyLock<PathBuf> = LazyLock::new(|| current_dir().unwrap_or_default());

pub static HOME_DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::home_dir().unwrap_or_default());
pub static HK_STATE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    var_path("HK_STATE_DIR").unwrap_or(
        dirs::state_dir()
            .unwrap_or(HOME_DIR.join(".local").join("state"))
            .join("hk"),
    )
});
pub static HK_FILE: LazyLock<Option<String>> = LazyLock::new(|| var("HK_FILE").ok());
pub static HK_CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    var_path("HK_CACHE_DIR").unwrap_or(
        dirs::cache_dir()
            .unwrap_or(HOME_DIR.join(".cache"))
            .join("hk"),
    )
});
pub static HK_LOG: LazyLock<log::LevelFilter> = LazyLock::new(|| {
    var_log_level("HK_LOG")
        .or(var_log_level("HK_LOG_LEVEL"))
        .unwrap_or(log::LevelFilter::Info)
});
pub static HK_LOG_FILE_LEVEL: LazyLock<log::LevelFilter> =
    LazyLock::new(|| var_log_level("HK_LOG_FILE_LEVEL").unwrap_or(*HK_LOG));
pub static HK_LOG_FILE: LazyLock<PathBuf> =
    LazyLock::new(|| var_path("HK_LOG_FILE").unwrap_or(HK_STATE_DIR.join("hk.log")));

// When set, write a JSON timing report to this path after the hook finishes
pub static HK_TIMING_JSON: LazyLock<Option<PathBuf>> = LazyLock::new(|| var_path("HK_TIMING_JSON"));

pub static HK_LIBGIT2: LazyLock<bool> = LazyLock::new(|| !var_false("HK_LIBGIT2"));
pub static HK_HIDE_WHEN_DONE: LazyLock<bool> = LazyLock::new(|| var_true("HK_HIDE_WHEN_DONE"));
pub static HK_CHECK_FIRST: LazyLock<bool> = LazyLock::new(|| !var_false("HK_CHECK_FIRST"));
pub static HK_STASH: LazyLock<Option<StashMethod>> = LazyLock::new(|| {
    if var_false("HK_STASH") {
        Some(StashMethod::None)
    } else {
        var("HK_STASH")
            .map(|v| Some(v.parse().expect("invalid HK_STASH value")))
            .unwrap_or(None)
    }
});
pub static HK_STASH_UNTRACKED: LazyLock<bool> = LazyLock::new(|| !var_false("HK_STASH_UNTRACKED"));
pub static HK_FIX: LazyLock<bool> = LazyLock::new(|| !var_false("HK_FIX"));
pub static HK_MISE: LazyLock<bool> = LazyLock::new(|| var_true("HK_MISE"));
pub static HK_SKIP_STEPS: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    var_csv("HK_SKIP_STEPS")
        .or(var_csv("HK_SKIP_STEP"))
        .unwrap_or_default()
});

// When true, allow output summaries to be printed in text mode
pub static HK_SUMMARY_TEXT: LazyLock<bool> = LazyLock::new(|| var_true("HK_SUMMARY_TEXT"));

// Cache control - defaults to enabled in release builds, disabled in debug builds
// Can be overridden with HK_CACHE=1 or HK_CACHE=0
pub static HK_CACHE: LazyLock<bool> = LazyLock::new(|| {
    var("HK_CACHE")
        .map(|_| !var_false("HK_CACHE"))
        .unwrap_or(!cfg!(debug_assertions)) // Default: enabled in release, disabled in debug
});

// Tracing configuration
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TraceMode {
    Off,
    Text,
    Json,
}

pub static HK_TRACE: LazyLock<TraceMode> =
    LazyLock::new(|| match var("HK_TRACE").map(|v| v.to_lowercase()) {
        Ok(v) if v == "json" => TraceMode::Json,
        Ok(v) if v == "1" || v == "true" => TraceMode::Text,
        _ => TraceMode::Off,
    });

pub static HK_JSON: LazyLock<bool> = LazyLock::new(|| var_true("HK_JSON"));

pub static GIT_INDEX_FILE: LazyLock<Option<PathBuf>> = LazyLock::new(|| var_path("GIT_INDEX_FILE"));

/// System's ARG_MAX value, memoized for performance
pub static ARG_MAX: LazyLock<usize> = LazyLock::new(|| {
    #[cfg(unix)]
    {
        // Try to get the system's ARG_MAX using sysconf
        unsafe {
            let value = libc::sysconf(libc::_SC_ARG_MAX);
            if value > 0 {
                return value as usize;
            }
        }
    }

    // Fallback: Use a conservative 128KB limit that works on most systems
    // This is much smaller than typical ARG_MAX values (often 256KB-2MB)
    // but safe for portability
    128 * 1024
});

fn var_path(name: &str) -> Option<PathBuf> {
    var(name).map(PathBuf::from).ok()
}

fn var_csv(name: &str) -> Option<IndexSet<String>> {
    var(name)
        .map(|val| val.split(',').map(|s| s.trim().to_string()).collect())
        .ok()
}

fn var_log_level(name: &str) -> Option<log::LevelFilter> {
    var(name).ok().and_then(|level| level.parse().ok())
}

fn var_true(name: &str) -> bool {
    var(name)
        .map(|val| val.to_lowercase())
        .map(|val| val == "true" || val == "1")
        .unwrap_or(false)
}

fn var_false(name: &str) -> bool {
    var(name)
        .map(|val| val.to_lowercase())
        .map(|val| val == "false" || val == "0")
        .unwrap_or(false)
}
