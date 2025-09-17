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
    inner: generated::settings::GeneratedSettings,
}

impl Settings {
    // Expose commonly used fields with computed logic where needed
    pub fn jobs(&self) -> NonZero<usize> {
        NonZero::new(self.inner.jobs).unwrap_or(NonZero::new(1).unwrap())
    }

    pub fn enabled_profiles(&self) -> IndexSet<String> {
        // Extract enabled profiles (those not starting with '!')
        self.inner
            .profiles
            .iter()
            .filter(|p| !p.starts_with('!'))
            .cloned()
            .collect()
    }

    pub fn disabled_profiles(&self) -> IndexSet<String> {
        // Extract disabled profiles (those starting with '!')
        self.inner
            .profiles
            .iter()
            .filter(|p| p.starts_with('!'))
            .map(|p| p.strip_prefix('!').unwrap().to_string())
            .collect()
    }

    pub fn fail_fast(&self) -> bool {
        self.inner.fail_fast
    }

    pub fn display_skip_reasons(&self) -> HashSet<String> {
        self.inner.display_skip_reasons.iter().cloned().collect()
    }

    pub fn warnings(&self) -> &IndexSet<String> {
        &self.inner.warnings
    }

    pub fn exclude(&self) -> &IndexSet<String> {
        &self.inner.exclude
    }

    pub fn skip_steps(&self) -> &IndexSet<String> {
        &self.inner.skip_steps
    }

    pub fn skip_hooks(&self) -> &IndexSet<String> {
        &self.inner.skip_hooks
    }

    pub fn all(&self) -> bool {
        self.inner.all
    }

    // Provide access to the full generated settings
    pub fn inner(&self) -> &generated::settings::GeneratedSettings {
        &self.inner
    }
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

        // Start with the generated default
        let mut inner = generated::settings::GeneratedSettings::default();

        // Handle profiles with proper precedence: CLI > env > defaults
        let mut all_profiles: IndexSet<String> = IndexSet::new();

        // Start with environment profiles
        all_profiles.extend(env::HK_PROFILE.iter().cloned());

        // Apply CLI profile overrides (union semantics)
        if let Some(ref cli_profiles) = override_settings.profiles {
            all_profiles.extend(cli_profiles.iter().cloned());
        }
        inner.profiles = all_profiles;

        // Handle display_skip_reasons with precedence
        if let Some(ref reasons) = override_settings.display_skip_reasons {
            inner.display_skip_reasons = reasons.clone();
        }

        // Handle hide_warnings with union semantics
        let mut hide_warnings = override_settings
            .hide_warnings
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment hide_warnings (union semantics)
        hide_warnings.extend(env::HK_HIDE_WARNINGS.iter().cloned());
        inner.hide_warnings = hide_warnings;

        // Handle warnings, filtering out hidden ones
        let mut warnings = override_settings
            .warnings
            .as_ref()
            .cloned()
            .unwrap_or_default();
        warnings.retain(|tag| !inner.hide_warnings.contains(tag));
        inner.warnings = warnings;

        // Handle exclude with union semantics
        let mut exclude = override_settings
            .exclude
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment excludes (union semantics)
        exclude.extend(env::HK_EXCLUDE.iter().cloned());
        inner.exclude = exclude;

        // Handle skip_steps with union semantics
        let mut skip_steps = override_settings
            .skip_steps
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment skip_steps (union semantics)
        skip_steps.extend(env::HK_SKIP_STEPS.iter().cloned());
        inner.skip_steps = skip_steps;

        // Handle skip_hooks with union semantics
        let mut skip_hooks = override_settings
            .skip_hooks
            .as_ref()
            .cloned()
            .unwrap_or_default();
        // Always add environment skip_hooks (union semantics)
        skip_hooks.extend(env::HK_SKIP_HOOK.iter().cloned());
        inner.skip_hooks = skip_hooks;

        // Handle jobs with precedence: CLI > env > default
        let jobs_value = override_settings.jobs.unwrap_or_else(|| env::HK_JOBS.get());
        inner.jobs = jobs_value;

        // Handle fail_fast with precedence: CLI > env > default
        if let Some(fail_fast) = override_settings.fail_fast.or_else(|| *env::HK_FAIL_FAST) {
            inner.fail_fast = fail_fast;
        }

        // Handle all with precedence: CLI > default
        if let Some(all) = override_settings.all {
            inner.all = all;
        }

        Self { inner }
    }
}
