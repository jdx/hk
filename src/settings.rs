use std::{
    collections::HashSet,
    num::NonZero,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};

use indexmap::IndexSet;

use crate::env;

// Include the generated settings structs from the build
pub mod generated {
    pub mod settings {
        include!(concat!(env!("OUT_DIR"), "/generated_settings.rs"));
    }
    pub mod settings_override {
        include!(concat!(env!("OUT_DIR"), "/generated_settings_override.rs"));
    }
    pub mod cli {
        include!(concat!(env!("OUT_DIR"), "/generated_cli_flags.rs"));
    }
    pub mod settings_meta {
        include!(concat!(env!("OUT_DIR"), "/generated_settings_meta.rs"));
    }

    // Re-export the main types for convenience
    pub use settings_meta::*;
    pub use settings_override::GeneratedSettingsOverride;
}

#[derive(Debug)]
pub struct Settings {
    pub jobs: NonZero<usize>,
    pub enabled_profiles: IndexSet<String>,
    pub disabled_profiles: IndexSet<String>,
    pub fail_fast: bool,
    pub display_skip_reasons: HashSet<String>,
    pub warnings: IndexSet<String>,
    pub exclude: IndexSet<String>,
    pub skip_steps: IndexSet<String>,
    pub skip_hooks: IndexSet<String>,
    pub all: bool,
}

// Global storage for programmatically set settings
// We store deltas that override the base settings from all other sources
static SETTINGS_OVERRIDE: LazyLock<Mutex<generated::GeneratedSettingsOverride>> =
    LazyLock::new(|| Mutex::new(generated::GeneratedSettingsOverride::default()));

impl Settings {
    pub fn get() -> Settings {
        Settings::default()
    }

    pub fn with_profiles(profiles: &[String]) {
        let mut override_settings = SETTINGS_OVERRIDE.lock().unwrap();

        for profile in profiles {
            if profile.starts_with('!') {
                let profile = profile.strip_prefix('!').unwrap();
                // Add to disabled profiles
                if override_settings.profiles.is_none() {
                    override_settings.profiles = Some(IndexSet::new());
                }
                // Remove from enabled profiles (if present)
                if let Some(ref mut enabled) = override_settings.profiles {
                    enabled.shift_remove(profile);
                }
                // This will be handled in Default implementation
            } else {
                // Add to enabled profiles
                if override_settings.profiles.is_none() {
                    override_settings.profiles = Some(IndexSet::new());
                }
                if let Some(ref mut profiles_set) = override_settings.profiles {
                    profiles_set.insert(profile.to_string());
                }
            }
        }
    }

    pub fn get_user_config_path() -> Option<PathBuf> {
        SETTINGS_OVERRIDE.lock().unwrap().hkrc.clone()
    }

    pub fn set_jobs(jobs: NonZero<usize>) {
        SETTINGS_OVERRIDE.lock().unwrap().jobs = Some(jobs.get());
    }

    pub fn set_user_config_path(path: PathBuf) {
        SETTINGS_OVERRIDE.lock().unwrap().hkrc = Some(path);
    }

    pub fn set_fail_fast(fail_fast: bool) {
        SETTINGS_OVERRIDE.lock().unwrap().fail_fast = Some(fail_fast);
    }

    pub fn set_all(all: bool) {
        SETTINGS_OVERRIDE.lock().unwrap().all = Some(all);
    }

    pub fn set_fix(fix: bool) {
        SETTINGS_OVERRIDE.lock().unwrap().fix = Some(fix);
    }

    pub fn set_check(check: bool) {
        SETTINGS_OVERRIDE.lock().unwrap().check = Some(check);
    }

    pub fn set_display_skip_reasons(display_skip_reasons: HashSet<String>) {
        SETTINGS_OVERRIDE.lock().unwrap().display_skip_reasons =
            Some(display_skip_reasons.into_iter().collect());
    }

    pub fn set_warnings(warnings: IndexSet<String>) {
        SETTINGS_OVERRIDE.lock().unwrap().warnings = Some(warnings);
    }

    pub fn set_hide_warnings(hide_warnings: IndexSet<String>) {
        SETTINGS_OVERRIDE.lock().unwrap().hide_warnings = Some(hide_warnings);
    }

    pub fn add_exclude<I, S>(patterns: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut override_settings = SETTINGS_OVERRIDE.lock().unwrap();
        if override_settings.exclude.is_none() {
            override_settings.exclude = Some(IndexSet::new());
        }
        if let Some(ref mut exclude) = override_settings.exclude {
            for pattern in patterns {
                exclude.insert(pattern.as_ref().to_string());
            }
        }
    }

    pub fn add_skip_steps<I, S>(steps: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut override_settings = SETTINGS_OVERRIDE.lock().unwrap();
        if override_settings.skip_steps.is_none() {
            override_settings.skip_steps = Some(IndexSet::new());
        }
        if let Some(ref mut skip_steps) = override_settings.skip_steps {
            for step in steps {
                skip_steps.insert(step.as_ref().to_string());
            }
        }
    }

    pub fn add_skip_hooks<I, S>(hooks: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut override_settings = SETTINGS_OVERRIDE.lock().unwrap();
        if override_settings.skip_hooks.is_none() {
            override_settings.skip_hooks = Some(IndexSet::new());
        }
        if let Some(ref mut skip_hooks) = override_settings.skip_hooks {
            for hook in hooks {
                skip_hooks.insert(hook.as_ref().to_string());
            }
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let override_settings = SETTINGS_OVERRIDE.lock().unwrap();

        // Handle profiles with proper precedence: CLI > env > defaults
        let mut all_profiles: IndexSet<String> = IndexSet::new();

        // Start with environment profiles
        all_profiles.extend(env::HK_PROFILE.iter().cloned());

        // Apply CLI profile overrides (union semantics)
        if let Some(ref cli_profiles) = override_settings.profiles {
            all_profiles.extend(cli_profiles.iter().cloned());
        }

        // Separate enabled and disabled profiles
        let disabled_profiles: IndexSet<String> = all_profiles
            .iter()
            .filter(|p| p.starts_with('!'))
            .map(|p| p.strip_prefix('!').unwrap().to_string())
            .collect();

        let enabled_profiles: IndexSet<String> = all_profiles
            .iter()
            .filter(|p| !p.starts_with('!'))
            .filter(|p| !disabled_profiles.contains(*p))
            .map(|p| p.to_string())
            .collect();

        // Handle display_skip_reasons with precedence
        let display_skip_reasons: HashSet<String> = override_settings
            .display_skip_reasons
            .as_ref()
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_else(|| {
                // Default: only profile-not-enabled is shown
                let mut set = HashSet::new();
                set.insert("profile-not-enabled".to_string());
                set
            });

        // Handle hide_warnings with union semantics
        let mut hide_warnings = override_settings
            .hide_warnings
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment hide_warnings (union semantics)
        hide_warnings.extend(env::HK_HIDE_WARNINGS.iter().cloned());

        // Handle warnings, filtering out hidden ones
        let warnings = override_settings
            .warnings
            .as_ref()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|tag| !hide_warnings.contains(tag))
            .collect();

        // Handle exclude with union semantics
        let mut exclude = override_settings
            .exclude
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment excludes (union semantics)
        exclude.extend(env::HK_EXCLUDE.iter().cloned());

        // Handle skip_steps with union semantics
        let mut skip_steps = override_settings
            .skip_steps
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment skip_steps (union semantics)
        skip_steps.extend(env::HK_SKIP_STEPS.iter().cloned());

        // Handle skip_hooks with union semantics
        let mut skip_hooks = override_settings
            .skip_hooks
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment skip_hooks (union semantics)
        skip_hooks.extend(env::HK_SKIP_HOOK.iter().cloned());

        // Handle jobs with precedence: CLI > env > default
        let jobs = override_settings
            .jobs
            .map(|j| NonZero::new(j).unwrap_or(*env::HK_JOBS))
            .unwrap_or(*env::HK_JOBS);

        // Handle fail_fast with precedence: CLI > env > default
        let fail_fast = override_settings
            .fail_fast
            .or_else(|| *env::HK_FAIL_FAST)
            .unwrap_or(true);

        // Handle all with precedence: CLI > default
        let all = override_settings.all.unwrap_or(false);

        Self {
            jobs,
            enabled_profiles,
            disabled_profiles,
            fail_fast,
            display_skip_reasons,
            warnings,
            exclude,
            skip_steps,
            skip_hooks,
            all,
        }
    }
}
