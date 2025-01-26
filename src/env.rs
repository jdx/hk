pub use std::env::*;
use std::{path::PathBuf, sync::LazyLock};

// pub static ANGLER_BIN: LazyLock<PathBuf> =
//     LazyLock::new(|| current_exe().unwrap().canonicalize().unwrap());
// pub static CWD: LazyLock<PathBuf> = LazyLock::new(|| current_dir().unwrap_or_default());

pub static HOME_DIR: LazyLock<PathBuf> = LazyLock::new(|| dirs::home_dir().unwrap_or_default());
pub static ANGLER_STATE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    var_path("ANGLER_STATE_DIR").unwrap_or(
        dirs::state_dir()
            .unwrap_or(HOME_DIR.join(".local").join("state"))
            .join("angler"),
    )
});
pub static ANGLER_CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    var_path("ANGLER_CACHE_DIR").unwrap_or(
        dirs::cache_dir()
            .unwrap_or(HOME_DIR.join(".cache"))
            .join("angler"),
    )
});
pub static ANGLER_LOG: LazyLock<log::LevelFilter> =
    LazyLock::new(|| var_log_level("ANGLER_LOG").unwrap_or(log::LevelFilter::Info));
pub static ANGLER_LOG_FILE_LEVEL: LazyLock<log::LevelFilter> =
    LazyLock::new(|| var_log_level("ANGLER_LOG_FILE_LEVEL").unwrap_or(*ANGLER_LOG));
pub static ANGLER_LOG_FILE: LazyLock<PathBuf> =
    LazyLock::new(|| var_path("ANGLER_LOG_FILE").unwrap_or(ANGLER_STATE_DIR.join("angler.log")));

fn var_path(name: &str) -> Option<PathBuf> {
    var(name).map(PathBuf::from).ok()
}

fn var_log_level(name: &str) -> Option<log::LevelFilter> {
    var(name).ok().and_then(|level| level.parse().ok())
}

// fn var_false(name: &str) -> bool {
//     var(name)
//         .map(|val| val.to_lowercase())
//         .map(|val| val == "false" || val == "0")
//         .unwrap_or(false)
// }
