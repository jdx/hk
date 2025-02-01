pub use std::env::*;
use std::{path::PathBuf, sync::LazyLock};

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
pub static HK_CACHE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    var_path("HK_CACHE_DIR").unwrap_or(
        dirs::cache_dir()
            .unwrap_or(HOME_DIR.join(".cache"))
            .join("hk"),
    )
});
pub static HK_LOG: LazyLock<log::LevelFilter> =
    LazyLock::new(|| var_log_level("HK_LOG").unwrap_or(log::LevelFilter::Info));
pub static HK_LOG_FILE_LEVEL: LazyLock<log::LevelFilter> =
    LazyLock::new(|| var_log_level("HK_LOG_FILE_LEVEL").unwrap_or(*HK_LOG));
pub static HK_LOG_FILE: LazyLock<PathBuf> =
    LazyLock::new(|| var_path("HK_LOG_FILE").unwrap_or(HK_STATE_DIR.join("hk.log")));

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
