use crate::settings::Settings;
use git2::Config;
use indexmap::IndexSet;
use std::num::NonZero;
use std::path::PathBuf;

pub fn read_git_config() -> Result<(), git2::Error> {
    let config = Config::open_default()?;

    // Read jobs
    if let Ok(jobs) = config.get_i32("hk.jobs") {
        if jobs > 0 {
            if let Some(jobs) = NonZero::new(jobs as usize) {
                Settings::set_jobs(jobs);
            }
        }
    }

    // Read profiles
    if let Ok(profiles) = read_string_list(&config, "hk.profile") {
        Settings::with_profiles(&profiles.into_iter().collect::<Vec<_>>());
    }

    // Read fail_fast
    if let Ok(fail_fast) = config.get_bool("hk.failFast") {
        Settings::set_fail_fast(fail_fast);
    }

    // Read fix
    if let Ok(fix) = config.get_bool("hk.fix") {
        Settings::set_fix(fix);
    }

    // Read check
    if let Ok(check) = config.get_bool("hk.check") {
        Settings::set_check(check);
    }

    // Read stash method
    if let Ok(stash) = config.get_string("hk.stash") {
        if let Ok(method) = stash.parse::<crate::git::StashMethod>() {
            // TODO: Add Settings::set_stash when we add the stash field to Settings
            _ = method; // Silence warning for now
        }
    }

    // Read stashUntracked
    if let Ok(stash_untracked) = config.get_bool("hk.stashUntracked") {
        // TODO: Add Settings::set_stash_untracked when we add the field
        _ = stash_untracked; // Silence warning for now
    }

    // Read checkFirst
    if let Ok(check_first) = config.get_bool("hk.checkFirst") {
        // TODO: Add Settings::set_check_first when we add the field
        _ = check_first; // Silence warning for now
    }

    // Read json/trace
    if let Ok(json) = config.get_bool("hk.json") {
        // TODO: Add Settings::set_json when we add the field
        _ = json; // Silence warning for now
    }

    if let Ok(trace) = config.get_bool("hk.trace") {
        // TODO: Add Settings::set_trace when we add the field
        _ = trace; // Silence warning for now
    }

    // Read warnings/hideWarnings
    if let Ok(warnings) = read_string_list(&config, "hk.warnings") {
        Settings::set_warnings(warnings);
    }

    if let Ok(hide_warnings) = read_string_list(&config, "hk.hideWarnings") {
        Settings::set_hide_warnings(hide_warnings);
    }

    // Read excludes
    if let Ok(excludes) = read_string_list(&config, "hk.exclude") {
        let paths: Vec<PathBuf> = excludes.into_iter().map(PathBuf::from).collect();
        Settings::add_exclude_paths(paths);
    }

    if let Ok(exclude_globs) = read_string_list(&config, "hk.excludeGlob") {
        Settings::add_exclude_globs(exclude_globs.into_iter().collect::<Vec<_>>());
    }

    Ok(())
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
