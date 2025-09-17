use crate::settings::{Settings, generated::SETTINGS_META};
use git2::{Config, Repository};
use indexmap::IndexSet;
use std::num::NonZero;

pub fn read_git_config() -> Result<(), git2::Error> {
    // Try to find repository config first, fall back to default
    let config = if let Ok(repo) = Repository::open_from_env() {
        repo.config()?
    } else {
        Config::open_default()?
    };

    // Dynamically process each setting that has git sources
    for (setting_name, setting_meta) in SETTINGS_META.iter() {
        for git_key in setting_meta.sources.git {
            match setting_meta.typ {
                "bool" => {
                    if let Ok(value) = config.get_bool(git_key) {
                        apply_bool_setting(setting_name, value);
                    }
                }
                "usize" => {
                    if let Ok(value) = config.get_i32(git_key) {
                        if *setting_name == "jobs" && value > 0 {
                            if let Some(jobs) = NonZero::new(value as usize) {
                                Settings::set_jobs(jobs);
                            }
                        }
                    }
                }
                "string" | "enum" => {
                    if let Ok(value) = config.get_string(git_key) {
                        apply_string_setting(setting_name, &value);
                    }
                }
                "list<string>" => {
                    if let Ok(values) = read_string_list(&config, git_key) {
                        apply_string_list_setting(setting_name, values);
                    }
                }
                _ => {
                    // Handle other types as needed
                }
            }
        }
    }

    Ok(())
}

fn apply_bool_setting(setting_name: &str, value: bool) {
    match setting_name {
        "fail_fast" => Settings::set_fail_fast(value),
        "fix" => Settings::set_fix(value),
        "check" => Settings::set_check(value),
        "all" => Settings::set_all(value),
        _ => {
            // Settings not yet implemented in Settings struct
        }
    }
}

fn apply_string_setting(setting_name: &str, value: &str) {
    match setting_name {
        "stash" => {
            if let Ok(method) = value.parse::<crate::git::StashMethod>() {
                // TODO: Add Settings::set_stash when we add the stash field to Settings
                _ = method; // Silence warning for now
            }
        }
        _ => {
            // Other string settings not yet implemented
        }
    }
}

fn apply_string_list_setting(setting_name: &str, values: IndexSet<String>) {
    match setting_name {
        "profiles" => {
            Settings::with_profiles(&values.into_iter().collect::<Vec<_>>());
        }
        "warnings" => {
            Settings::set_warnings(values);
        }
        "hide_warnings" => {
            Settings::set_hide_warnings(values);
        }
        "exclude" => {
            Settings::add_exclude(values.into_iter().collect::<Vec<_>>());
        }
        "skip_steps" => {
            Settings::add_skip_steps(values.into_iter().collect::<Vec<_>>());
        }
        "skip_hooks" => {
            Settings::add_skip_hooks(values.into_iter().collect::<Vec<_>>());
        }
        _ => {
            // Other list settings not yet implemented
        }
    }
}

fn read_string_list(config: &Config, key: &str) -> Result<IndexSet<String>, git2::Error> {
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
