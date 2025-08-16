use std::{
    collections::HashSet,
    num::NonZero,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use indexmap::IndexSet;

use crate::env;

#[derive(Debug)]
pub struct Settings {
    pub jobs: NonZero<usize>,
    pub enabled_profiles: IndexSet<String>,
    pub disabled_profiles: IndexSet<String>,
    pub fail_fast: bool,
    pub display_skip_reasons: HashSet<String>,
}

static JOBS: LazyLock<Mutex<Option<NonZero<usize>>>> = LazyLock::new(Default::default);
static ENABLED_PROFILES: LazyLock<Mutex<Option<IndexSet<String>>>> =
    LazyLock::new(Default::default);
static DISABLED_PROFILES: LazyLock<Mutex<Option<IndexSet<String>>>> =
    LazyLock::new(Default::default);
static USER_CONFIG_PATH: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(Default::default);
static FAIL_FAST: LazyLock<Mutex<Option<bool>>> = LazyLock::new(Default::default);
static ALL: LazyLock<Mutex<Option<bool>>> = LazyLock::new(Default::default);
static FIX: LazyLock<Mutex<Option<bool>>> = LazyLock::new(Default::default);
static CHECK: LazyLock<Mutex<Option<bool>>> = LazyLock::new(Default::default);
static DISPLAY_SKIP_REASONS: LazyLock<Mutex<Option<HashSet<String>>>> =
    LazyLock::new(Default::default);

impl Settings {
    pub fn get() -> Settings {
        Settings::default()
    }

    pub fn with_profiles(profiles: &[String]) {
        for profile in profiles {
            if profile.starts_with('!') {
                let profile = profile.strip_prefix('!').unwrap();
                let mut disabled_profiles = DISABLED_PROFILES.lock().unwrap();
                disabled_profiles
                    .get_or_insert_default()
                    .insert(profile.to_string());
            } else {
                let mut enabled_profiles = ENABLED_PROFILES.lock().unwrap();
                enabled_profiles
                    .get_or_insert_default()
                    .insert(profile.to_string());
                let mut disabled_profiles = DISABLED_PROFILES.lock().unwrap();
                disabled_profiles
                    .get_or_insert_default()
                    .shift_remove(profile);
            }
        }
    }

    pub fn get_user_config_path() -> Option<PathBuf> {
        USER_CONFIG_PATH.lock().unwrap().clone()
    }

    pub fn set_jobs(jobs: NonZero<usize>) {
        *JOBS.lock().unwrap() = Some(jobs);
    }

    pub fn set_user_config_path(path: PathBuf) {
        *USER_CONFIG_PATH.lock().unwrap() = Some(path);
    }

    pub fn set_fail_fast(fail_fast: bool) {
        *FAIL_FAST.lock().unwrap() = Some(fail_fast);
    }

    pub fn set_all(all: bool) {
        *ALL.lock().unwrap() = Some(all);
    }

    pub fn set_fix(fix: bool) {
        *FIX.lock().unwrap() = Some(fix);
    }

    pub fn set_check(check: bool) {
        *CHECK.lock().unwrap() = Some(check);
    }

    pub fn set_display_skip_reasons(display_skip_reasons: HashSet<String>) {
        *DISPLAY_SKIP_REASONS.lock().unwrap() = Some(display_skip_reasons);
    }
}

impl Default for Settings {
    fn default() -> Self {
        let disabled_profiles: IndexSet<String> = DISABLED_PROFILES
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                env::HK_PROFILE
                    .iter()
                    .filter(|p| p.starts_with('!'))
                    .map(|p| p.strip_prefix('!').unwrap().to_string())
                    .collect()
            });
        let enabled_profiles: IndexSet<String> =
            ENABLED_PROFILES.lock().unwrap().clone().unwrap_or_else(|| {
                env::HK_PROFILE
                    .iter()
                    .filter(|p| !disabled_profiles.contains(*p))
                    .filter(|p| !p.starts_with('!'))
                    .map(|p| p.to_string())
                    .collect()
            });
        let display_skip_reasons =
            DISPLAY_SKIP_REASONS
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| {
                    // Default: only profile-not-enabled is shown
                    let mut set = HashSet::new();
                    set.insert("profile-not-enabled".to_string());
                    set
                });
        Self {
            jobs: JOBS.lock().unwrap().unwrap_or(*env::HK_JOBS),
            enabled_profiles,
            disabled_profiles,
            fail_fast: FAIL_FAST.lock().unwrap().unwrap_or(*env::HK_FAIL_FAST),
            display_skip_reasons,
        }
    }
}
