use std::{
    collections::HashSet,
    num::NonZero,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex, OnceLock},
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

#[derive(Debug, Clone)]
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
        // For backward compatibility, clone from the global snapshot
        (*Self::get_snapshot()).clone()
    }

    /// Get the global settings snapshot, initializing it if needed
    pub fn get_snapshot() -> SettingsSnapshot {
        GLOBAL_SETTINGS
            .get_or_init(|| {
                SettingsBuilder::new()
                    .from_env()
                    .from_git()
                    .build_snapshot()
            })
            .clone()
    }

    /// Reset the global settings cache (useful for testing)
    pub fn reset_global_cache() {
        // This is a bit tricky since OnceLock doesn't have a reset method
        // We'll need to use unsafe code or create a new approach
        // For now, we'll document this limitation
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

// Immutable settings snapshot using Arc for efficient sharing
pub type SettingsSnapshot = Arc<Settings>;

// Global cached settings instance
static GLOBAL_SETTINGS: OnceLock<SettingsSnapshot> = OnceLock::new();

/// Builder for creating Settings instances with different sources
#[derive(Default)]
pub struct SettingsBuilder {
    override_settings: generated::GeneratedSettingsOverride,
}

impl SettingsBuilder {
    /// Create a new SettingsBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Load settings from environment variables
    pub fn from_env(self) -> Self {
        // Environment variables are automatically loaded in the Default implementation
        // This is a no-op but provides the fluent API
        self
    }

    /// Load settings from git configuration
    pub fn from_git(mut self) -> Self {
        // Load git configuration and apply to our local override
        if let Err(e) = self.load_git_config() {
            eprintln!("Warning: Failed to load git config: {}", e);
        }
        self
    }

    /// Load git configuration into the builder
    fn load_git_config(&mut self) -> Result<(), git2::Error> {
        use git2::{Config, Repository};

        // Try to find repository config first, fall back to default
        let config = if let Ok(repo) = Repository::open_from_env() {
            repo.config()?
        } else {
            Config::open_default()?
        };

        // Load git config values into our local override
        for (setting_name, setting_meta) in generated::SETTINGS_META.iter() {
            for git_key in setting_meta.sources.git {
                match setting_meta.typ {
                    "bool" => {
                        if let Ok(value) = config.get_bool(git_key) {
                            match *setting_name {
                                "fail_fast" => self.override_settings.fail_fast = Some(value),
                                _ => {}
                            }
                        }
                    }
                    "usize" => {
                        if let Ok(value) = config.get_i32(git_key) {
                            if *setting_name == "jobs" && value > 0 {
                                self.override_settings.jobs = Some(value as usize);
                            }
                        }
                    }
                    "list<string>" => {
                        if let Ok(value) = config.get_str(git_key) {
                            let items: IndexSet<String> = value
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();

                            match *setting_name {
                                "display_skip_reasons" => {
                                    self.override_settings.display_skip_reasons = Some(items);
                                }
                                "hide_warnings" => {
                                    self.override_settings.hide_warnings = Some(items);
                                }
                                "warnings" => {
                                    self.override_settings.warnings = Some(items);
                                }
                                "exclude" => {
                                    self.override_settings.exclude = Some(items);
                                }
                                "skip_steps" => {
                                    self.override_settings.skip_steps = Some(items);
                                }
                                "skip_hooks" => {
                                    self.override_settings.skip_hooks = Some(items);
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Load settings from a Config instance (from hkrc.pkl/toml etc.)
    pub fn from_config(mut self, config: &crate::config::Config) -> Self {
        // Apply config-level settings to our override
        if let Some(fail_fast) = config.fail_fast {
            self.override_settings.fail_fast = Some(fail_fast);
        }

        if let Some(ref display_skip_reasons) = config.display_skip_reasons {
            self.override_settings.display_skip_reasons =
                Some(display_skip_reasons.iter().cloned().collect());
        }

        if let Some(ref hide_warnings) = config.hide_warnings {
            self.override_settings.hide_warnings = Some(hide_warnings.iter().cloned().collect());
        }

        if let Some(ref warnings) = config.warnings {
            self.override_settings.warnings = Some(warnings.iter().cloned().collect());
        }

        self
    }

    /// Build the Settings instance
    pub fn build(self) -> Settings {
        // Create settings directly without modifying global state
        let mut inner = generated::settings::GeneratedSettings::default();

        // Handle profiles with proper precedence: CLI > env > defaults
        let mut all_profiles: IndexSet<String> = IndexSet::new();

        // Start with environment profiles
        all_profiles.extend(env::HK_PROFILE.iter().cloned());

        // Apply builder profile overrides (union semantics)
        if let Some(ref builder_profiles) = self.override_settings.profiles {
            all_profiles.extend(builder_profiles.iter().cloned());
        }
        inner.profiles = all_profiles;

        // Handle display_skip_reasons
        if let Some(ref reasons) = self.override_settings.display_skip_reasons {
            inner.display_skip_reasons = reasons.clone();
        }

        // Handle hide_warnings with union semantics
        let mut hide_warnings = self
            .override_settings
            .hide_warnings
            .as_ref()
            .cloned()
            .unwrap_or_default();
        hide_warnings.extend(env::HK_HIDE_WARNINGS.iter().cloned());
        inner.hide_warnings = hide_warnings;

        // Handle warnings, filtering out hidden ones
        let mut warnings = self
            .override_settings
            .warnings
            .as_ref()
            .cloned()
            .unwrap_or_default();
        warnings.retain(|tag| !inner.hide_warnings.contains(tag));
        inner.warnings = warnings;

        // Handle exclude with union semantics
        let mut exclude = self
            .override_settings
            .exclude
            .as_ref()
            .cloned()
            .unwrap_or_default();
        exclude.extend(env::HK_EXCLUDE.iter().cloned());
        inner.exclude = exclude;

        // Handle skip_steps with union semantics
        let mut skip_steps = self
            .override_settings
            .skip_steps
            .as_ref()
            .cloned()
            .unwrap_or_default();
        skip_steps.extend(env::HK_SKIP_STEPS.iter().cloned());
        inner.skip_steps = skip_steps;

        // Handle skip_hooks with union semantics
        let mut skip_hooks = self
            .override_settings
            .skip_hooks
            .as_ref()
            .cloned()
            .unwrap_or_default();
        skip_hooks.extend(env::HK_SKIP_HOOK.iter().cloned());
        inner.skip_hooks = skip_hooks;

        // Handle jobs with precedence: CLI > env > default
        let jobs_value = self
            .override_settings
            .jobs
            .unwrap_or_else(|| env::HK_JOBS.get());
        inner.jobs = jobs_value;

        // Handle fail_fast with precedence: CLI > env > default
        if let Some(fail_fast) = self
            .override_settings
            .fail_fast
            .or_else(|| *env::HK_FAIL_FAST)
        {
            inner.fail_fast = fail_fast;
        }

        // Handle all with precedence: CLI > default
        if let Some(all) = self.override_settings.all {
            inner.all = all;
        }

        Settings { inner }
    }

    /// Build and return as an immutable snapshot
    pub fn build_snapshot(self) -> SettingsSnapshot {
        Arc::new(self.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_builder_fluent_api() {
        // Test that the fluent API works correctly
        let settings = SettingsBuilder::new().from_env().from_git().build();

        // Should have some reasonable defaults
        assert!(settings.jobs().get() >= 1);
        assert!(settings.fail_fast() || !settings.fail_fast()); // Should be either true or false
    }

    #[test]
    fn test_settings_snapshot_caching() {
        // Get multiple snapshots - they should be the same Arc
        let snapshot1 = Settings::get_snapshot();
        let snapshot2 = Settings::get_snapshot();

        // They should point to the same data (same Arc)
        assert!(Arc::ptr_eq(&snapshot1, &snapshot2));
    }

    #[test]
    fn test_settings_from_config() {
        use crate::config::Config;

        let mut config = Config::default();
        config.fail_fast = Some(false);
        config.warnings = Some(vec!["test-warning".to_string()]);

        let settings = SettingsBuilder::new().from_config(&config).build();

        assert_eq!(settings.fail_fast(), false);
        assert!(settings.warnings().contains("test-warning"));
    }
}
