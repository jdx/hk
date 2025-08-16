use std::{
    num::NonZero,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use indexmap::{IndexMap, IndexSet};

use crate::env;

#[derive(Debug)]
pub struct Settings {
    pub jobs: NonZero<usize>,
    pub enabled_profiles: IndexSet<String>,
    pub disabled_profiles: IndexSet<String>,
    pub fail_fast: bool,
    pub skip_reasons: IndexMap<String, bool>,
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
static SKIP_REASONS: LazyLock<Mutex<Option<IndexMap<String, bool>>>> =
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

    pub fn set_skip_reasons(skip_reasons: IndexMap<String, bool>) {
        *SKIP_REASONS.lock().unwrap() = Some(skip_reasons);
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
        let skip_reasons = SKIP_REASONS.lock().unwrap().clone().unwrap_or_else(|| {
            // Default: only ProfileNotEnabled is shown
            let mut map = IndexMap::new();
            map.insert("ProfileNotEnabled".to_string(), true);
            map.insert("ProfileExplicitlyDisabled".to_string(), false);
            map.insert("NoCommandForRunType".to_string(), false);
            map.insert("Env".to_string(), false);
            map.insert("Cli".to_string(), false);
            map
        });
        Self {
            jobs: JOBS.lock().unwrap().unwrap_or(*env::HK_JOBS),
            enabled_profiles,
            disabled_profiles,
            fail_fast: FAIL_FAST.lock().unwrap().unwrap_or(*env::HK_FAIL_FAST),
            skip_reasons,
        }
    }
}
