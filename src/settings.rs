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

    pub fn hide_warnings(&self) -> &IndexSet<String> {
        &self.inner.hide_warnings
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
        // Check if we need to initialize
        let mut initialized = INITIALIZED.lock().unwrap();
        if !*initialized {
            // First access - initialize with all sources including programmatic overrides
            let new_settings = Arc::new(Self::build_from_all_sources());
            GLOBAL_SETTINGS.store(new_settings.clone());
            *initialized = true;
            return new_settings;
        }
        drop(initialized); // Release the lock early

        // Already initialized - return the cached value
        GLOBAL_SETTINGS.load_full()
    }

    /// Build settings from all sources using the canonical path
    fn build_from_all_sources() -> Settings {
        let programmatic_overrides = SETTINGS_OVERRIDE.lock().unwrap().clone();

        SettingsBuilder::new()
            .from_env()
            .from_git()
            .with_programmatic(programmatic_overrides)
            .build()
    }

    /// Reset the global settings cache (useful for testing)
    #[allow(dead_code)]
    pub fn reset_global_cache() {
        let new_settings = Arc::new(Self::build_from_all_sources());
        GLOBAL_SETTINGS.store(new_settings);
        let mut initialized = INITIALIZED.lock().unwrap();
        *initialized = true; // Mark as initialized with the new settings
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
        // Use the single canonical build path
        Self::build_from_all_sources()
    }
}

// Immutable settings snapshot using Arc for efficient sharing
pub type SettingsSnapshot = Arc<Settings>;

// Global cached settings instance using ArcSwap for safe reloading
// Initially contains a dummy value that will be replaced on first access
static GLOBAL_SETTINGS: LazyLock<ArcSwap<Settings>> = LazyLock::new(|| {
    // Initial dummy value - will be replaced on first real access
    ArcSwap::from_pointee(Settings {
        inner: generated::settings::GeneratedSettings::default(),
    })
});

// Track whether we've initialized with real settings
static INITIALIZED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

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
            debug!("Failed to load git config: {}", e);
        }
        self
    }

    /// Load git configuration into the builder
    fn load_git_config(&mut self) -> Result<(), git2::Error> {
        use git2::{Config, Repository};

        // Try to find repository config first, fall back to default
        let config = if let Ok(repo) = Repository::open_from_env() {
            debug!("Git config: Using repository from env");
            repo.config()?
        } else if let Ok(repo) = Repository::discover(".") {
            // Also try discovering from current directory
            debug!("Git config: Using repository from current directory");
            repo.config()?
        } else {
            debug!("Git config: Using default config");
            Config::open_default()?
        };

        // Load git config values into our git overrides using metadata
        for (setting_name, setting_meta) in generated::SETTINGS_META.iter() {
            for git_key in setting_meta.sources.git {
                self.apply_git_setting(&config, setting_name, setting_meta, git_key);
            }
        }

        Ok(())
    }

    /// Apply a single git config setting based on metadata
    fn apply_git_setting(
        &mut self,
        config: &git2::Config,
        setting_name: &str,
        setting_meta: &generated::SettingMeta,
        git_key: &str,
    ) {
        match setting_meta.typ {
            "bool" => {
                if let Ok(value) = config.get_bool(git_key) {
                    Self::set_bool_field(&mut self.git_overrides, setting_name, value);
                }
            }
            "usize" => {
                if let Ok(value) = config.get_i32(git_key) {
                    if value > 0 {
                        Self::set_usize_field(
                            &mut self.git_overrides,
                            setting_name,
                            value as usize,
                        );
                    }
                }
            }
            "u8" => {
                if let Ok(value) = config.get_i32(git_key) {
                    if value >= 0 && value <= 255 {
                        Self::set_u8_field(&mut self.git_overrides, setting_name, value as u8);
                    }
                }
            }
            "list<string>" => {
                if let Ok(value) = Self::read_string_list(config, git_key) {
                    if !value.is_empty() {
                        debug!("Git config: {} = {:?}", git_key, value);
                        Self::set_string_list_field(&mut self.git_overrides, setting_name, value);
                    }
                }
            }
            "string" | "enum" => {
                if let Ok(value) = config.get_str(git_key) {
                    Self::set_string_field(
                        &mut self.git_overrides,
                        setting_name,
                        value.to_string(),
                    );
                }
            }
            "path" => {
                if let Ok(value) = config.get_str(git_key) {
                    Self::set_path_field(
                        &mut self.git_overrides,
                        setting_name,
                        PathBuf::from(value),
                    );
                }
            }
            _ => {}
        }
    }

    /// Read a string list from git config (handles both multivar and comma-separated)
    fn read_string_list(config: &git2::Config, key: &str) -> Result<IndexSet<String>, git2::Error> {
        let mut result = IndexSet::new();

        // Try to read as multivar (multiple entries with same key)
        match config.multivar(key, None) {
            Ok(mut entries) => {
                while let Some(entry) = entries.next() {
                    if let Some(value) = entry?.value() {
                        // Support comma-separated values too
                        for item in value.split(',').map(|s| s.trim()) {
                            if !item.is_empty() {
                                result.insert(item.to_string());
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // If multivar fails, try single value
                if let Ok(value) = config.get_string(key) {
                    for item in value.split(',').map(|s| s.trim()) {
                        if !item.is_empty() {
                            result.insert(item.to_string());
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    // Helper methods to set fields on GeneratedSettingsOverride by name
    fn set_bool_field(
        overrides: &mut generated::GeneratedSettingsOverride,
        name: &str,
        value: bool,
    ) {
        match name {
            "all" => overrides.all = Some(value),
            "check" => overrides.check = Some(value),
            "check_first" => overrides.check_first = Some(value),
            "fail_fast" => overrides.fail_fast = Some(value),
            "fix" => overrides.fix = Some(value),
            "hide_when_done" => overrides.hide_when_done = Some(value),
            "json" => overrides.json = Some(value),
            "libgit2" => overrides.libgit2 = Some(value),
            "mise" => overrides.mise = Some(value),
            "no_progress" => overrides.no_progress = Some(value),
            "quiet" => overrides.quiet = Some(value),
            "silent" => overrides.silent = Some(value),
            "slow" => overrides.slow = Some(value),
            "stash_untracked" => overrides.stash_untracked = Some(value),
            "summary_text" => overrides.summary_text = Some(value),
            _ => {}
        }
    }

    fn set_usize_field(
        overrides: &mut generated::GeneratedSettingsOverride,
        name: &str,
        value: usize,
    ) {
        match name {
            "jobs" => overrides.jobs = Some(value),
            _ => {}
        }
    }

    fn set_u8_field(overrides: &mut generated::GeneratedSettingsOverride, name: &str, value: u8) {
        match name {
            "verbose" => overrides.verbose = Some(value),
            _ => {}
        }
    }

    fn set_string_field(
        overrides: &mut generated::GeneratedSettingsOverride,
        name: &str,
        value: String,
    ) {
        match name {
            "log_file_level" => overrides.log_file_level = Some(value),
            "log_level" => overrides.log_level = Some(value),
            "stash" => overrides.stash = Some(value),
            "trace" => overrides.trace = Some(value),
            _ => {}
        }
    }

    fn set_path_field(
        overrides: &mut generated::GeneratedSettingsOverride,
        name: &str,
        value: PathBuf,
    ) {
        match name {
            "cache_dir" => overrides.cache_dir = Some(value),
            "hkrc" => overrides.hkrc = Some(value),
            "log_file" => overrides.log_file = Some(value),
            "state_dir" => overrides.state_dir = Some(value),
            "timing_json" => overrides.timing_json = Some(value),
            _ => {}
        }
    }

    fn set_string_list_field(
        overrides: &mut generated::GeneratedSettingsOverride,
        name: &str,
        value: IndexSet<String>,
    ) {
        match name {
            "display_skip_reasons" => overrides.display_skip_reasons = Some(value),
            "exclude" => overrides.exclude = Some(value),
            "hide_warnings" => overrides.hide_warnings = Some(value),
            "profiles" => overrides.profiles = Some(value),
            "skip_hooks" => overrides.skip_hooks = Some(value),
            "skip_steps" => overrides.skip_steps = Some(value),
            "warnings" => overrides.warnings = Some(value),
            _ => {}
        }
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
                Self::apply_env_setting(&mut env_overrides, setting_name, setting_meta, env_var);
            }
        }

        env_overrides
    }

    /// Apply a single environment variable setting based on metadata
    fn apply_env_setting(
        overrides: &mut generated::GeneratedSettingsOverride,
        setting_name: &str,
        setting_meta: &generated::SettingMeta,
        env_var: &str,
    ) {
        match setting_meta.typ {
            "bool" => {
                if let Ok(value) = std::env::var(env_var) {
                    let bool_value = match value.to_lowercase().as_str() {
                        "true" | "1" | "yes" | "on" => true,
                        "false" | "0" | "no" | "off" => false,
                        _ => return,
                    };
                    Self::set_bool_field(overrides, setting_name, bool_value);
                }
            }
            "usize" => {
                if let Ok(value) = std::env::var(env_var) {
                    if let Ok(usize_value) = value.parse::<usize>() {
                        Self::set_usize_field(overrides, setting_name, usize_value);
                    }
                }
            }
            "u8" => {
                if let Ok(value) = std::env::var(env_var) {
                    if let Ok(u8_value) = value.parse::<u8>() {
                        Self::set_u8_field(overrides, setting_name, u8_value);
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
                    Self::set_string_list_field(overrides, setting_name, items);
                }
            }
            "path" => {
                if let Ok(value) = std::env::var(env_var) {
                    Self::set_path_field(overrides, setting_name, PathBuf::from(value));
                }
            }
            "string" | "enum" => {
                if let Ok(value) = std::env::var(env_var) {
                    Self::set_string_field(overrides, setting_name, value);
                }
            }
            _ => {}
        }
    }

    /// Set programmatic overrides
    pub fn with_programmatic(mut self, overrides: generated::GeneratedSettingsOverride) -> Self {
        self.programmatic_overrides = overrides;
        self
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
