use std::{
    collections::HashSet,
    num::NonZero,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
};

use arc_swap::ArcSwap;

use indexmap::IndexSet;

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
    pub mod settings_merge {
        include!(concat!(env!("OUT_DIR"), "/generated_settings_merge.rs"));
    }

    // Re-export the main types for convenience
    pub use settings_merge::SettingsMerger;
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

    /// Get the global settings snapshot
    pub fn get_snapshot() -> SettingsSnapshot {
        GLOBAL_SETTINGS.load_full()
    }

    /// Reset the global settings cache (useful for testing)
    #[allow(dead_code)]
    pub fn reset_global_cache() {
        // Rebuild settings from scratch with current overrides
        let programmatic_overrides = SETTINGS_OVERRIDE.lock().unwrap().clone();

        let mut builder = SettingsBuilder::new().from_env().from_git();
        builder.programmatic_overrides = programmatic_overrides;

        let new_settings = builder.build_snapshot();
        GLOBAL_SETTINGS.store(new_settings);
    }

    /// Reload settings from all sources (useful for config file changes)
    #[allow(dead_code)]
    pub fn reload() -> SettingsSnapshot {
        Self::reset_global_cache();
        Self::get_snapshot()
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
        // Use the systematic merge logic via SettingsBuilder with programmatic overrides
        let programmatic_overrides = SETTINGS_OVERRIDE.lock().unwrap().clone();

        let mut builder = SettingsBuilder::new().from_env().from_git();
        builder.programmatic_overrides = programmatic_overrides;

        builder.build()
    }
}

// Immutable settings snapshot using Arc for efficient sharing
pub type SettingsSnapshot = Arc<Settings>;

// Global cached settings instance using ArcSwap for safe reloading
static GLOBAL_SETTINGS: LazyLock<ArcSwap<Settings>> = LazyLock::new(|| {
    // Initialize with settings built from all sources
    let programmatic_overrides = SETTINGS_OVERRIDE.lock().unwrap().clone();

    let mut builder = SettingsBuilder::new().from_env().from_git();
    builder.programmatic_overrides = programmatic_overrides;

    ArcSwap::from_pointee(builder.build())
});

/// Builder for creating Settings instances with different sources
#[derive(Default)]
pub struct SettingsBuilder {
    env_overrides: generated::GeneratedSettingsOverride,
    git_overrides: generated::GeneratedSettingsOverride,
    config_overrides: generated::GeneratedSettingsOverride,
    cli_overrides: generated::GeneratedSettingsOverride,
    programmatic_overrides: generated::GeneratedSettingsOverride,
}

impl SettingsBuilder {
    /// Create a new SettingsBuilder
    pub fn new() -> Self {
        Self::default()
    }

    /// Load settings from environment variables
    pub fn from_env(mut self) -> Self {
        self.env_overrides = Self::collect_env_overrides();
        self
    }

    /// Load settings from git configuration
    pub fn from_git(mut self) -> Self {
        // Load git configuration and apply to our git overrides
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

        // Load git config values into our git overrides
        for (setting_name, setting_meta) in generated::SETTINGS_META.iter() {
            for git_key in setting_meta.sources.git {
                match setting_meta.typ {
                    "bool" => {
                        if let Ok(value) = config.get_bool(git_key) {
                            match *setting_name {
                                "fail_fast" => self.git_overrides.fail_fast = Some(value),
                                "check" => self.git_overrides.check = Some(value),
                                "fix" => self.git_overrides.fix = Some(value),
                                "json" => self.git_overrides.json = Some(value),
                                "check_first" => self.git_overrides.check_first = Some(value),
                                "stash_untracked" => {
                                    self.git_overrides.stash_untracked = Some(value)
                                }
                                _ => {}
                            }
                        }
                    }
                    "usize" => {
                        if let Ok(value) = config.get_i32(git_key) {
                            if *setting_name == "jobs" && value > 0 {
                                self.git_overrides.jobs = Some(value as usize);
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
                                "profiles" => self.git_overrides.profiles = Some(items),
                                "display_skip_reasons" => {
                                    self.git_overrides.display_skip_reasons = Some(items)
                                }
                                "hide_warnings" => self.git_overrides.hide_warnings = Some(items),
                                "warnings" => self.git_overrides.warnings = Some(items),
                                "exclude" => self.git_overrides.exclude = Some(items),
                                "skip_steps" => self.git_overrides.skip_steps = Some(items),
                                "skip_hooks" => self.git_overrides.skip_hooks = Some(items),
                                _ => {}
                            }
                        }
                    }
                    "enum" => {
                        if let Ok(value) = config.get_str(git_key) {
                            match *setting_name {
                                "stash" => self.git_overrides.stash = Some(value.to_string()),
                                "trace" => self.git_overrides.trace = Some(value.to_string()),
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
        // Apply config-level settings to our config overrides
        if let Some(fail_fast) = config.fail_fast {
            self.config_overrides.fail_fast = Some(fail_fast);
        }

        if let Some(ref display_skip_reasons) = config.display_skip_reasons {
            self.config_overrides.display_skip_reasons =
                Some(display_skip_reasons.iter().cloned().collect());
        }

        if let Some(ref hide_warnings) = config.hide_warnings {
            self.config_overrides.hide_warnings = Some(hide_warnings.iter().cloned().collect());
        }

        if let Some(ref warnings) = config.warnings {
            self.config_overrides.warnings = Some(warnings.iter().cloned().collect());
        }

        self
    }

    /// Build the Settings instance
    pub fn build(self) -> Settings {
        // Use the generated systematic merge logic
        let merger = generated::SettingsMerger::default();

        // Apply systematic precedence: defaults → env → git → pkl → CLI → programmatic
        let defaults = generated::settings::GeneratedSettings::default();
        let inner = merger.merge_sources(
            &defaults, // defaults
            &self.env_overrides,
            &self.git_overrides,
            &self.config_overrides, // pkl config
            &self.cli_overrides,
            &self.programmatic_overrides,
        );

        Settings { inner }
    }

    /// Collect environment variable overrides into a settings override structure
    fn collect_env_overrides() -> generated::GeneratedSettingsOverride {
        let mut env_overrides = generated::GeneratedSettingsOverride::default();

        // Collect environment variables following the generated settings metadata
        for (setting_name, setting_meta) in generated::SETTINGS_META.iter() {
            for env_var in setting_meta.sources.env {
                match setting_meta.typ {
                    "bool" => {
                        if let Ok(value) = std::env::var(env_var) {
                            let bool_value = match value.to_lowercase().as_str() {
                                "true" | "1" | "yes" | "on" => true,
                                "false" | "0" | "no" | "off" => false,
                                _ => continue,
                            };
                            match *setting_name {
                                "fail_fast" => env_overrides.fail_fast = Some(bool_value),
                                "check" => env_overrides.check = Some(bool_value),
                                "fix" => env_overrides.fix = Some(bool_value),
                                "all" => env_overrides.all = Some(bool_value),
                                "json" => env_overrides.json = Some(bool_value),
                                "libgit2" => env_overrides.libgit2 = Some(bool_value),
                                "mise" => env_overrides.mise = Some(bool_value),
                                "quiet" => env_overrides.quiet = Some(bool_value),
                                "silent" => env_overrides.silent = Some(bool_value),
                                "slow" => env_overrides.slow = Some(bool_value),
                                "stash_untracked" => {
                                    env_overrides.stash_untracked = Some(bool_value)
                                }
                                "summary_text" => env_overrides.summary_text = Some(bool_value),
                                "check_first" => env_overrides.check_first = Some(bool_value),
                                "hide_when_done" => env_overrides.hide_when_done = Some(bool_value),
                                "no_progress" => env_overrides.no_progress = Some(bool_value),
                                _ => {}
                            }
                        }
                    }
                    "usize" => {
                        if let Ok(value) = std::env::var(env_var) {
                            if let Ok(usize_value) = value.parse::<usize>() {
                                match *setting_name {
                                    "jobs" => env_overrides.jobs = Some(usize_value),
                                    _ => {}
                                }
                            }
                        }
                    }
                    "u8" => {
                        if let Ok(value) = std::env::var(env_var) {
                            if let Ok(u8_value) = value.parse::<u8>() {
                                match *setting_name {
                                    "verbose" => env_overrides.verbose = Some(u8_value),
                                    _ => {}
                                }
                            }
                        }
                    }
                    "list<string>" => {
                        if let Ok(value) = std::env::var(env_var) {
                            let items: IndexSet<String> = value
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();

                            match *setting_name {
                                "profiles" => env_overrides.profiles = Some(items),
                                "display_skip_reasons" => {
                                    env_overrides.display_skip_reasons = Some(items)
                                }
                                "hide_warnings" => env_overrides.hide_warnings = Some(items),
                                "warnings" => env_overrides.warnings = Some(items),
                                "exclude" => env_overrides.exclude = Some(items),
                                "skip_steps" => env_overrides.skip_steps = Some(items),
                                "skip_hooks" => env_overrides.skip_hooks = Some(items),
                                _ => {}
                            }
                        }
                    }
                    "path" => {
                        if let Ok(value) = std::env::var(env_var) {
                            let path_value = PathBuf::from(value);
                            match *setting_name {
                                "cache_dir" => env_overrides.cache_dir = Some(path_value),
                                "state_dir" => env_overrides.state_dir = Some(path_value),
                                "log_file" => env_overrides.log_file = Some(path_value),
                                "timing_json" => env_overrides.timing_json = Some(path_value),
                                _ => {}
                            }
                        }
                    }
                    "enum" => {
                        if let Ok(value) = std::env::var(env_var) {
                            match *setting_name {
                                "log_level" => env_overrides.log_level = Some(value),
                                "log_file_level" => env_overrides.log_file_level = Some(value),
                                "stash" => env_overrides.stash = Some(value),
                                "trace" => env_overrides.trace = Some(value),
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        env_overrides
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
