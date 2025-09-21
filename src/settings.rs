use std::{
    num::NonZero,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    thread,
};

use arc_swap::ArcSwap;
use generated::merge::{SettingValue, SourceMap};
use indexmap::IndexSet;
use once_cell::sync::Lazy;
use serde_json::json;

// Include the generated settings structs from the build
pub mod generated {
    pub mod settings {
        include!(concat!(env!("OUT_DIR"), "/generated_settings.rs"));
    }
    pub mod settings_meta {
        include!(concat!(env!("OUT_DIR"), "/generated_settings_meta.rs"));
    }
    pub mod merge {
        include!(concat!(env!("OUT_DIR"), "/generated_settings_merge.rs"));
    }
    // no generated accessors

    // Re-export the main types for convenience
    pub use settings_meta::*;
}
pub use generated::settings::Settings;

use crate::settings::generated::merge::SettingSource;

#[macro_export]
macro_rules! setting {
    ($field:ident) => {{
        let __s = $crate::settings::Settings::get();
        __s.inner().$field.clone()
    }};
}

#[derive(Debug, Clone, Default)]
pub struct CliSnapshot {
    pub hkrc: Option<PathBuf>,
    pub jobs: Option<usize>,
    pub profiles: Vec<String>,
    pub slow: bool,
    pub quiet: bool,
    pub silent: bool,
}

static CLI_SNAPSHOT: Lazy<Mutex<Option<CliSnapshot>>> = Lazy::new(|| Mutex::new(None));
static PROGRAMMATIC_MAP: Lazy<Mutex<SourceMap>> = Lazy::new(|| Mutex::new(SourceMap::new()));

fn read_git_string_list(config: &git2::Config, key: &str) -> Result<IndexSet<String>, git2::Error> {
    let mut result = IndexSet::new();
    match config.multivar(key, None) {
        Ok(mut entries) => {
            while let Some(entry) = entries.next() {
                if let Some(value) = entry?.value() {
                    for item in value.split(',').map(|s| s.trim()) {
                        if !item.is_empty() {
                            result.insert(item.to_string());
                        }
                    }
                }
            }
        }
        Err(_) => {
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

impl Settings {
    pub fn set_cli_snapshot(snapshot: CliSnapshot) {
        let mut guard = CLI_SNAPSHOT.lock().unwrap();
        *guard = Some(snapshot);
    }

    pub fn cli_user_config_path() -> Option<PathBuf> {
        CLI_SNAPSHOT
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|s| s.hkrc.clone())
    }
    pub fn get() -> Settings {
        // For backward compatibility, clone from the global snapshot
        (*Self::get_snapshot()).clone()
    }

    /// Get the global settings snapshot
    fn get_snapshot() -> SettingsSnapshot {
        // Check if we need to initialize
        let mut initialized = INITIALIZED.lock().unwrap();
        if !*initialized {
            // First access - initialize with all sources including programmatic overrides
            let new_settings = Arc::new(Self::build_from_all_sources().unwrap_or_else(|err| {
                eprintln!("Warning: Failed to load configuration: {}", err);
                generated::settings::Settings::default()
            }));
            GLOBAL_SETTINGS.store(new_settings.clone());
            *initialized = true;
            return new_settings;
        }
        drop(initialized); // Release the lock early

        // Already initialized - return the cached value
        GLOBAL_SETTINGS.load_full()
    }

    // Expose commonly used fields with computed logic where needed
    pub fn jobs(&self) -> NonZero<usize> {
        NonZero::new(self.jobs).unwrap_or(thread::available_parallelism().unwrap())
    }

    pub fn enabled_profiles(&self) -> IndexSet<String> {
        // Extract enabled profiles (those not starting with '!')
        self.profiles
            .iter()
            .filter(|p| !p.starts_with('!'))
            .cloned()
            .collect()
    }

    pub fn disabled_profiles(&self) -> IndexSet<String> {
        // Extract disabled profiles (those starting with '!')
        self.profiles
            .iter()
            .filter(|p| p.starts_with('!'))
            .map(|p| p.strip_prefix('!').unwrap().to_string())
            .collect()
    }

    /// Build settings from all sources using the canonical path
    fn build_from_all_sources() -> Result<Settings, eyre::Error> {
        let defaults = generated::settings::Settings::default();
        let env_map = Self::collect_env_map();
        let git_map = Self::collect_git_map()?;
        let pkl_map = Self::collect_pkl_map()?;
        let cli_map = Self::collect_cli_map();
        Ok(Self::merge_settings_generic(
            &defaults, &env_map, &git_map, &pkl_map, &cli_map,
        ))
    }

    pub(crate) fn merge_settings_generic(
        defaults: &generated::settings::Settings,
        env: &SourceMap,
        git: &SourceMap,
        pkl: &SourceMap,
        cli: &SourceMap,
    ) -> generated::settings::Settings {
        let mut val = serde_json::to_value(defaults.clone()).unwrap_or_else(|_| json!({}));
        // helper to replace scalar value
        fn set_value(val: &mut serde_json::Value, field: &str, v: &SettingValue) {
            let new_v = match v {
                SettingValue::Bool(b) => json!(b),
                SettingValue::Usize(n) => json!(n),
                SettingValue::U8(n) => json!(n),
                SettingValue::String(s) => json!(s),
                SettingValue::Path(p) => json!(p.display().to_string()),
                SettingValue::StringList(list) => json!(list.iter().collect::<Vec<_>>()),
            };
            if let Some(obj) = val.as_object_mut() {
                obj.insert(field.to_string(), new_v);
            }
        }
        // helper to union list<string>
        fn union_list(val: &mut serde_json::Value, field: &str, list: &indexmap::IndexSet<String>) {
            let mut current: indexmap::IndexSet<String> =
                if let Some(arr) = val.get(field).and_then(|v| v.as_array()) {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else {
                    indexmap::IndexSet::new()
                };
            current.extend(list.iter().cloned());
            if let Some(obj) = val.as_object_mut() {
                obj.insert(field.to_string(), json!(current.iter().collect::<Vec<_>>()));
            }
        }

        // Apply layers in precedence order (low to high): defaults < pkl < git < env < cli
        for (name, meta) in generated::SETTINGS_META.iter() {
            let field = *name;
            let merge_is_union = meta.merge == Some("union");
            // closure to apply setting from one layer
            let mut apply = |map: &SourceMap| {
                if let Some(sv) = map.get(field) {
                    match sv {
                        SettingValue::StringList(set) if merge_is_union => {
                            union_list(&mut val, field, set)
                        }
                        _ => set_value(&mut val, field, sv),
                    }
                }
            };
            // Lowest precedence first; last applied wins
            apply(pkl);
            apply(git);
            apply(env);
            apply(cli);
        }

        serde_json::from_value(val).unwrap_or_else(|_| defaults.clone())
    }

    pub(crate) fn merge_settings_with_sources_generic(
        defaults: &generated::settings::Settings,
        env: &SourceMap,
        git: &SourceMap,
        pkl: &SourceMap,
        cli: &SourceMap,
    ) -> (
        generated::settings::Settings,
        generated::merge::SourceInfoMap,
    ) {
        let mut val = serde_json::to_value(defaults.clone()).unwrap_or_else(|_| json!({}));
        let mut info: generated::merge::SourceInfoMap = indexmap::IndexMap::new();

        fn set_value2(
            val: &mut serde_json::Value,
            info: &mut generated::merge::SourceInfoMap,
            field: &'static str,
            v: &SettingValue,
            src: SettingSource,
        ) {
            let new_v = match v {
                SettingValue::Bool(b) => json!(b),
                SettingValue::Usize(n) => json!(n),
                SettingValue::U8(n) => json!(n),
                SettingValue::String(s) => json!(s),
                SettingValue::Path(p) => json!(p.display().to_string()),
                SettingValue::StringList(list) => json!(list.iter().collect::<Vec<_>>()),
            };
            if let Some(obj) = val.as_object_mut() {
                obj.insert(field.to_string(), new_v);
            }
            info.entry(field).or_default().last = Some(src);
        }

        fn union_list2(
            val: &mut serde_json::Value,
            info: &mut generated::merge::SourceInfoMap,
            field: &'static str,
            list: &indexmap::IndexSet<String>,
            src: SettingSource,
        ) {
            let mut current: indexmap::IndexSet<String> =
                if let Some(arr) = val.get(field).and_then(|v| v.as_array()) {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                } else {
                    indexmap::IndexSet::new()
                };
            for item in list.iter() {
                let inserted = current.insert(item.clone());
                let entry = info.entry(field).or_default();
                let m = entry.list_items.get_or_insert_with(indexmap::IndexMap::new);
                let srcs = m.entry(item.clone()).or_default();
                // Always record source for item regardless of duplication, keeps full provenance
                srcs.push(src.clone());
                if inserted {
                    // nothing extra
                }
            }
            if let Some(obj) = val.as_object_mut() {
                obj.insert(field.to_string(), json!(current.iter().collect::<Vec<_>>()));
            }
            info.entry(field).or_default().last = Some(src);
        }

        // initialize defaults provenance
        for (name, meta) in generated::SETTINGS_META.iter() {
            let field = *name;
            if meta.typ.starts_with("list<string>") {
                if let Some(arr) = val.get(field).and_then(|v| v.as_array()) {
                    if !arr.is_empty() {
                        let mut m: indexmap::IndexMap<String, Vec<SettingSource>> =
                            indexmap::IndexMap::new();
                        for it in arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())) {
                            m.insert(it, vec![SettingSource::Defaults]);
                        }
                        info.entry(field).or_default().list_items = Some(m);
                    }
                }
            }
            info.entry(field).or_default().last = Some(SettingSource::Defaults);
        }

        // Apply layers in precedence order (low to high): defaults < pkl < git < env < cli
        for (name, meta) in generated::SETTINGS_META.iter() {
            let field = *name;
            let merge_is_union = meta.merge == Some("union");
            let mut apply = |map: &SourceMap, src: SettingSource| {
                if let Some(sv) = map.get(field) {
                    match sv {
                        SettingValue::StringList(set) if merge_is_union => {
                            union_list2(&mut val, &mut info, field, set, src)
                        }
                        _ => set_value2(&mut val, &mut info, field, sv, src),
                    }
                }
            };
            // Lowest precedence first; last applied wins
            apply(pkl, SettingSource::Pkl);
            apply(git, SettingSource::Git);
            apply(env, SettingSource::Env);
            apply(cli, SettingSource::Cli);
        }

        (
            serde_json::from_value(val).unwrap_or_else(|_| defaults.clone()),
            info,
        )
    }

    fn collect_env_map() -> SourceMap {
        let mut map: SourceMap = SourceMap::new();
        for (setting_name, meta) in generated::SETTINGS_META.iter() {
            for env_var in meta.sources.env {
                if let Ok(val) = std::env::var(env_var) {
                    match meta.typ {
                        "bool" => {
                            let s = val.to_lowercase();
                            let parsed = matches!(s.as_str(), "1" | "true" | "yes" | "on");
                            map.insert(setting_name, SettingValue::Bool(parsed));
                        }
                        "usize" => {
                            if let Ok(n) = val.parse::<usize>() {
                                map.insert(setting_name, SettingValue::Usize(n));
                            }
                        }
                        "u8" => {
                            if let Ok(n) = val.parse::<u8>() {
                                map.insert(setting_name, SettingValue::U8(n));
                            }
                        }
                        t if t.starts_with("list<string>") => {
                            let items: IndexSet<String> = val
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            map.insert(setting_name, SettingValue::StringList(items));
                        }
                        "path" => {
                            map.insert(setting_name, SettingValue::Path(PathBuf::from(val)));
                        }
                        "string" | "enum" => {
                            map.insert(setting_name, SettingValue::String(val));
                        }
                        _ => {}
                    }
                    break; // first matching env var wins
                }
            }
        }
        map
    }

    fn collect_git_map() -> Result<SourceMap, eyre::Error> {
        let mut map: SourceMap = SourceMap::new();
        let cfg = {
            use git2::{Config, Repository};
            if let Ok(repo) = Repository::open_from_env() {
                repo.config()
            } else if let Ok(repo) = Repository::discover(".") {
                repo.config()
            } else {
                Config::open_default()
            }
        }?;
        for (setting_name, meta) in generated::SETTINGS_META.iter() {
            let mut merged: Option<IndexSet<String>> = None;
            for key in meta.sources.git {
                match meta.typ {
                    "bool" => {
                        if let Ok(v) = cfg.get_bool(key) {
                            map.insert(setting_name, SettingValue::Bool(v));
                            break;
                        }
                    }
                    "usize" => {
                        if let Ok(v) = cfg.get_i32(key) {
                            if v > 0 {
                                map.insert(setting_name, SettingValue::Usize(v as usize));
                                break;
                            }
                        }
                    }
                    "u8" => {
                        if let Ok(v) = cfg.get_i32(key) {
                            if (0..=255).contains(&v) {
                                map.insert(setting_name, SettingValue::U8(v as u8));
                                break;
                            }
                        }
                    }
                    t if t.starts_with("list<string>") => {
                        if let Ok(list) = read_git_string_list(&cfg, key) {
                            if !list.is_empty() {
                                if let Some(acc) = &mut merged {
                                    acc.extend(list.into_iter());
                                } else {
                                    merged = Some(list);
                                }
                            }
                        }
                    }
                    "string" | "enum" => {
                        if let Ok(v) = cfg.get_str(key) {
                            map.insert(setting_name, SettingValue::String(v.to_string()));
                            break;
                        }
                    }
                    "path" => {
                        if let Ok(v) = cfg.get_str(key) {
                            map.insert(setting_name, SettingValue::Path(PathBuf::from(v)));
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if let Some(set) = merged {
                map.insert(setting_name, SettingValue::StringList(set));
            }
        }
        Ok(map)
    }

    fn collect_pkl_map() -> Result<SourceMap, eyre::Error> {
        let mut map: SourceMap = SourceMap::new();
        let cfg = crate::config::Config::get()?;
        // Convert config to JSON for dynamic field access
        let config_json = serde_json::to_value(&cfg)?;
        // Iterate over all settings that have PKL sources
        for (setting_name, meta) in generated::SETTINGS_META.iter() {
            if !meta.sources.pkl.is_empty() {
                // Convert setting_name from kebab-case to snake_case for JSON field lookup
                let field_name = setting_name.replace('-', "_");

                if let Some(value) = config_json.get(&field_name) {
                    // Convert JSON value to SettingValue based on type
                    match (meta.typ, value) {
                        ("bool", serde_json::Value::Bool(b)) => {
                            map.insert(setting_name, SettingValue::Bool(*b));
                        }
                        ("usize", serde_json::Value::Number(n)) => {
                            if let Some(u) = n.as_u64().and_then(|u| usize::try_from(u).ok()) {
                                map.insert(setting_name, SettingValue::Usize(u));
                            }
                        }
                        ("u8", serde_json::Value::Number(n)) => {
                            if let Some(u) = n.as_u64().and_then(|u| u8::try_from(u).ok()) {
                                map.insert(setting_name, SettingValue::U8(u));
                            }
                        }
                        ("string" | "enum", serde_json::Value::String(s)) => {
                            map.insert(setting_name, SettingValue::String(s.clone()));
                        }
                        ("path", serde_json::Value::String(s)) => {
                            map.insert(setting_name, SettingValue::Path(s.into()));
                        }
                        (typ, serde_json::Value::Array(arr)) if typ.starts_with("list<string>") => {
                            let strings: IndexSet<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect();
                            map.insert(setting_name, SettingValue::StringList(strings));
                        }
                        (typ, serde_json::Value::String(s)) if typ.starts_with("list<string>") => {
                            // Handle StringOrList serialized as a single string
                            let strings: IndexSet<String> = IndexSet::from([s.clone()]);
                            map.insert(setting_name, SettingValue::StringList(strings));
                        }
                        _ => {
                            // Skip values that don't match expected types or are null
                        }
                    }
                }
            }
        }
        Ok(map)
    }

    fn collect_cli_map() -> SourceMap {
        let mut map: SourceMap = SourceMap::new();
        if let Some(snapshot) = CLI_SNAPSHOT.lock().unwrap().clone() {
            if let Some(p) = snapshot.hkrc {
                map.insert("hkrc", SettingValue::Path(p));
            }
            if let Some(j) = snapshot.jobs {
                map.insert("jobs", SettingValue::Usize(j));
            }
            if !snapshot.profiles.is_empty() {
                let set: IndexSet<String> = snapshot.profiles.into_iter().collect();
                map.insert("profiles", SettingValue::StringList(set));
            }
            if snapshot.slow {
                map.insert("slow", SettingValue::Bool(true));
            }
            if snapshot.quiet {
                map.insert("quiet", SettingValue::Bool(true));
            }
            if snapshot.silent {
                map.insert("silent", SettingValue::Bool(true));
            }
        }
        // Apply programmatic map on top (highest precedence)
        {
            let prog = PROGRAMMATIC_MAP.lock().unwrap();
            for (k, v) in prog.iter() {
                match (map.get(k), v) {
                    (
                        Some(SettingValue::StringList(existing)),
                        SettingValue::StringList(additional),
                    ) => {
                        let mut merged = existing.clone();
                        merged.extend(additional.iter().cloned());
                        map.insert(*k, SettingValue::StringList(merged));
                    }
                    _ => {
                        map.insert(*k, v.clone());
                    }
                }
            }
        }
        map
    }
}

// Immutable settings snapshot using Arc for efficient sharing
pub type SettingsSnapshot = Arc<Settings>;

// Global cached settings instance using ArcSwap for safe reloading
// Initially contains a dummy value that will be replaced on first access
static GLOBAL_SETTINGS: LazyLock<ArcSwap<Settings>> = LazyLock::new(|| {
    // Initial dummy value - will be replaced on first real access
    ArcSwap::from_pointee(generated::settings::Settings::default())
});

// Track whether we've initialized with real settings
static INITIALIZED: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

impl Settings {
    /// Explain where a configuration value comes from, using collected source maps
    pub fn explain_value(key: &str) -> Result<String, eyre::Error> {
        use std::fmt::Write;

        let field_name = key.replace('-', "_");
        let meta = generated::SETTINGS_META
            .get(field_name.as_str())
            .ok_or_else(|| eyre::eyre!("Unknown configuration key: {}", key))?;

        let env_map = Self::collect_env_map();
        let git_map = Self::collect_git_map()?;
        let pkl_map = Self::collect_pkl_map()?;
        let cli_map = Self::collect_cli_map();

        // Use provenance-aware merge to get sources
        let defaults = generated::settings::Settings::default();
        let (_merged, sources) = Self::merge_settings_with_sources_generic(
            &defaults, &env_map, &git_map, &pkl_map, &cli_map,
        );

        // Determine exact identifiers (env var names, git keys, etc.) used for this field
        let env_id: Option<&'static str> = meta
            .sources
            .env
            .iter()
            .copied()
            .find(|name| std::env::var(name).is_ok());

        let git_id: Option<&'static str> = {
            use git2::{Config, Repository};
            let cfg_result = if let Ok(repo) = Repository::open_from_env() {
                repo.config()
            } else if let Ok(repo) = Repository::discover(".") {
                repo.config()
            } else {
                Config::open_default()
            };

            match cfg_result {
                Ok(cfg) => meta
                    .sources
                    .git
                    .iter()
                    .copied()
                    .find(|k| cfg.get_entry(k).is_ok()),
                Err(_) => None,
            }
        };

        let pkl_id: Option<&'static str> = if pkl_map.get(field_name.as_str()).is_some() {
            meta.sources.pkl.first().copied()
        } else {
            None
        };

        let cli_id: Option<&'static str> = if cli_map.get(field_name.as_str()).is_some() {
            meta.sources.cli.first().copied()
        } else {
            None
        };

        let source_to_string = |s: &generated::merge::SettingSource| -> String {
            match s {
                generated::merge::SettingSource::Defaults => "defaults".to_string(),
                generated::merge::SettingSource::Env => match env_id {
                    Some(id) => format!("env({})", id),
                    None => "env".to_string(),
                },
                generated::merge::SettingSource::Git => match git_id {
                    Some(id) => format!("git({})", id),
                    None => "git".to_string(),
                },
                generated::merge::SettingSource::Pkl => match pkl_id {
                    Some(id) => format!("pkl({})", id),
                    None => "pkl".to_string(),
                },
                generated::merge::SettingSource::Cli => match cli_id {
                    Some(id) => format!("cli({})", id),
                    None => "cli".to_string(),
                },
            }
        };

        fn display_value(v: &SettingValue) -> String {
            match v {
                SettingValue::Bool(b) => b.to_string(),
                SettingValue::Usize(n) => n.to_string(),
                SettingValue::U8(n) => n.to_string(),
                SettingValue::String(s) => s.clone(),
                SettingValue::Path(p) => p.display().to_string(),
                SettingValue::StringList(list) => format!("{:?}", list),
            }
        }

        let mut output = String::new();
        writeln!(
            &mut output,
            "Source resolution for '{}' (in precedence order):",
            key
        )?;
        writeln!(
            &mut output,
            "================================================"
        )?;

        // CLI
        if !meta.sources.cli.is_empty() {
            writeln!(&mut output, "  CLI FLAGS: {}", meta.sources.cli.join(", "))?;
            if let Some(v) = cli_map.get(field_name.as_str()) {
                writeln!(&mut output, "    ✓ Set to: {}", display_value(v))?;
            }
            if let Some(info) = sources.get(field_name.as_str()) {
                if let Some(src) = &info.last {
                    writeln!(&mut output, "    Source: {}", source_to_string(src))?;
                }
            }
        }

        // ENV
        if !meta.sources.env.is_empty() {
            writeln!(
                &mut output,
                "  ENVIRONMENT: {}",
                meta.sources.env.join(", ")
            )?;
            if let Some(v) = env_map.get(field_name.as_str()) {
                writeln!(&mut output, "    ✓ Set to: {}", display_value(v))?;
            }
            if let Some(info) = sources.get(field_name.as_str()) {
                if let Some(src) = &info.last {
                    writeln!(&mut output, "    Source: {}", source_to_string(src))?;
                }
            }
        }

        // GIT
        if !meta.sources.git.is_empty() {
            writeln!(&mut output, "  GIT CONFIG: {}", meta.sources.git.join(", "))?;
            if let Some(v) = git_map.get(field_name.as_str()) {
                writeln!(&mut output, "    ✓ Set to: {}", display_value(v))?;
            }
            if let Some(info) = sources.get(field_name.as_str()) {
                if let Some(src) = &info.last {
                    writeln!(&mut output, "    Source: {}", source_to_string(src))?;
                }
            }
        }

        // PKL
        if !meta.sources.pkl.is_empty() {
            writeln!(&mut output, "  PKL CONFIG: {}", meta.sources.pkl.join(", "))?;
            if let Some(v) = pkl_map.get(field_name.as_str()) {
                writeln!(&mut output, "    ✓ Set to: {}", display_value(v))?;
            }
            if let Some(info) = sources.get(field_name.as_str()) {
                if let Some(src) = &info.last {
                    writeln!(&mut output, "    Source: {}", source_to_string(src))?;
                }
            }
        }

        // Default
        writeln!(&mut output, "  DEFAULT:")?;
        if let Some(default) = &meta.default_value {
            writeln!(&mut output, "    Value: {}", default)?;
        } else {
            writeln!(&mut output, "    Value: (type default)")?;
        }

        // For list<string> types, show per-item provenance
        if meta.typ.starts_with("list<string>") {
            if let Some(info) = sources.get(field_name.as_str()) {
                if let Some(items) = &info.list_items {
                    writeln!(&mut output, "\n  Items and their sources:")?;
                    for (item, srcs) in items.iter() {
                        let parts: Vec<String> = srcs.iter().map(&source_to_string).collect();
                        writeln!(&mut output, "    - {}: {}", item, parts.join(", "))?;
                    }
                }
            }
        }

        writeln!(&mut output)?;
        writeln!(
            &mut output,
            "Merge strategy: {}",
            meta.merge.unwrap_or("replace")
        )?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // --- generated merge tests moved from config/test_generated_merge.rs ---
    use crate::settings::generated;
    use crate::settings::generated::merge::{SettingValue, SourceMap};

    #[test]
    fn test_settings_builder_fluent_api() {
        Settings::set_cli_snapshot(CliSnapshot {
            hkrc: Some(PathBuf::from(".hkrc.pkl")),
            ..Default::default()
        });
        // Test that the fluent API works correctly
        let settings = Settings::get();

        // Should have some reasonable defaults
        assert!(settings.jobs().get() >= 1);
    }

    #[test]
    fn test_settings_snapshot_caching() {
        Settings::set_cli_snapshot(CliSnapshot {
            hkrc: Some(PathBuf::from(".hkrc.pkl")),
            ..Default::default()
        });
        // Get multiple snapshots - they should be the same Arc
        let snapshot1 = Settings::get_snapshot();
        let snapshot2 = Settings::get_snapshot();

        // They should point to the same data (same Arc)
        assert!(Arc::ptr_eq(&snapshot1, &snapshot2));
    }

    #[test]
    fn test_settings_from_config() {
        Settings::set_cli_snapshot(CliSnapshot {
            hkrc: Some(PathBuf::from(".hkrc.pkl")),
            ..Default::default()
        });
        // Backwards-compatible behavior validated at higher level; smoke test get()
        let _settings = Settings::get();
    }

    fn set_list(map: &mut SourceMap, key: &'static str, vals: &[&str]) {
        let set: IndexSet<String> = vals.iter().map(|s| (*s).to_string()).collect();
        map.insert(key, SettingValue::StringList(set));
    }

    #[test]
    fn test_union_merge_skip_steps() {
        let defaults = generated::settings::Settings {
            skip_steps: IndexSet::from(["default_step".to_string()]),
            ..Default::default()
        };

        let mut env: SourceMap = SourceMap::new();
        let mut git: SourceMap = SourceMap::new();
        let mut pkl: SourceMap = SourceMap::new();
        let mut cli: SourceMap = SourceMap::new();

        set_list(&mut pkl, "skip_steps", &["pkl_step"]);
        set_list(&mut git, "skip_steps", &["git_step", "pkl_step"]);
        set_list(&mut env, "skip_steps", &["env_step"]);
        set_list(&mut cli, "skip_steps", &["cli_step", "env_step"]);

        let merged = Settings::merge_settings_generic(&defaults, &env, &git, &pkl, &cli);

        assert!(merged.skip_steps.contains("default_step"));
        assert!(merged.skip_steps.contains("pkl_step"));
        assert!(merged.skip_steps.contains("git_step"));
        assert!(merged.skip_steps.contains("env_step"));
        assert!(merged.skip_steps.contains("cli_step"));
        assert_eq!(merged.skip_steps.len(), 5);
    }

    #[test]
    fn test_replace_merge_fail_fast() {
        let defaults = generated::settings::Settings::default();

        let mut env: SourceMap = SourceMap::new();
        let mut git: SourceMap = SourceMap::new();
        let mut pkl: SourceMap = SourceMap::new();
        let mut cli: SourceMap = SourceMap::new();

        env.insert("fail_fast", SettingValue::Bool(false));
        git.insert("fail_fast", SettingValue::Bool(true));
        pkl.insert("fail_fast", SettingValue::Bool(false));
        cli.insert("fail_fast", SettingValue::Bool(true));

        let merged = Settings::merge_settings_generic(&defaults, &env, &git, &pkl, &cli);
        assert!(merged.fail_fast);
    }

    #[test]
    fn test_precedence_ordering_jobs() {
        let defaults = generated::settings::Settings::default();

        let mut env: SourceMap = SourceMap::new();
        let mut git: SourceMap = SourceMap::new();
        let mut pkl: SourceMap = SourceMap::new();
        let mut cli: SourceMap = SourceMap::new();

        pkl.insert("jobs", SettingValue::Usize(2));
        git.insert("jobs", SettingValue::Usize(3));
        env.insert("jobs", SettingValue::Usize(4));
        cli.insert("jobs", SettingValue::Usize(5));

        let merged = Settings::merge_settings_generic(&defaults, &env, &git, &pkl, &cli);
        assert_eq!(merged.jobs, 5);
    }

    #[test]
    fn test_precedence_with_missing_layers() {
        let defaults = generated::settings::Settings::default();

        let env: SourceMap = SourceMap::new();
        let mut git: SourceMap = SourceMap::new();
        let pkl: SourceMap = SourceMap::new();
        let cli: SourceMap = SourceMap::new();

        git.insert("jobs", SettingValue::Usize(3));

        let merged = Settings::merge_settings_generic(&defaults, &env, &git, &pkl, &cli);
        assert_eq!(merged.jobs, 3);
    }

    #[test]
    fn test_warnings_replace_merge() {
        let defaults = generated::settings::Settings {
            warnings: IndexSet::from(["warning1".to_string()]),
            ..Default::default()
        };

        let mut env: SourceMap = SourceMap::new();
        let git: SourceMap = SourceMap::new();
        let pkl: SourceMap = SourceMap::new();
        let cli: SourceMap = SourceMap::new();

        let env_set: IndexSet<String> = IndexSet::from(["warning4".to_string()]);
        env.insert("warnings", SettingValue::StringList(env_set));

        let merged = Settings::merge_settings_generic(&defaults, &env, &git, &pkl, &cli);
        assert!(merged.warnings.contains("warning4"));
        assert_eq!(merged.warnings.len(), 1);
    }
}
