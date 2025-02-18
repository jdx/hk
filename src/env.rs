pub use std::env::*;
use std::{num::NonZero, path::PathBuf, sync::LazyLock, thread};

use indexmap::IndexSet;

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

pub static HK_CHECK_FIRST: LazyLock<bool> = LazyLock::new(|| !var_false("HK_CHECK_FIRST"));
pub static HK_STASH: LazyLock<bool> = LazyLock::new(|| !var_false("HK_STASH"));
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
