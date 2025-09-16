pub use std::env::*;
use std::{num::NonZero, path::PathBuf, sync::LazyLock, thread};

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
pub static HK_PROFILE: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    var_csv("HK_PROFILE")
        .or(var_csv("HK_PROFILES"))
        .unwrap_or_default()
});
pub static HK_SKIP_STEPS: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    var_csv("HK_SKIP_STEPS")
        .or(var_csv("HK_SKIP_STEP"))
        .unwrap_or_default()
});
pub static HK_SKIP_HOOK: LazyLock<IndexSet<String>> = LazyLock::new(|| {
    var_csv("HK_SKIP_HOOK")
        .or(var_csv("HK_SKIP_HOOKS"))
        .unwrap_or_default()
});
pub static HK_JOBS: LazyLock<NonZero<usize>> = LazyLock::new(|| {
    var("HK_JOBS")
        .or(var("HK_JOB"))
        .ok()
        .and_then(|val| val.parse().ok())
        .or(thread::available_parallelism().ok())
        .unwrap_or(NonZero::new(4).unwrap())
});
// None means not set in environment; Some(true/false) if explicitly set
pub static HK_FAIL_FAST: LazyLock<Option<bool>> = LazyLock::new(|| var_bool("HK_FAIL_FAST"));
pub static HK_HIDE_WARNINGS: LazyLock<IndexSet<String>> =
    LazyLock::new(|| var_csv("HK_HIDE_WARNINGS").unwrap_or_default());

// Exclude paths - comma-separated list of paths to exclude from processing
pub static HK_EXCLUDE: LazyLock<IndexSet<String>> =
    LazyLock::new(|| var_csv("HK_EXCLUDE").unwrap_or_default());

// Exclude globs - comma-separated list of glob patterns to exclude
pub static HK_EXCLUDE_GLOB: LazyLock<IndexSet<String>> =
    LazyLock::new(|| var_csv("HK_EXCLUDE_GLOB").unwrap_or_default());

// When true, allow output summaries to be printed in text mode
pub static HK_SUMMARY_TEXT: LazyLock<bool> = LazyLock::new(|| var_true("HK_SUMMARY_TEXT"));

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

fn var_bool(name: &str) -> Option<bool> {
    match var(name) {
        Ok(val) => {
            let v = val.to_lowercase();
            match v.as_str() {
                "false" | "0" => Some(false),
                "true" | "1" => Some(true),
                _ => {
                    warn!("{name} is set to an invalid value: {val}");
                    None
                }
            }
        }
        Err(_) => None,
    }
}
