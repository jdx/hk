use demand::DemandOption;

use crate::Result;
use crate::builtins::{BUILTINS_META, BuiltinMeta};

use super::DEFAULT_HOOKS;
use super::detector::Detection;

/// Let user select builtins interactively
pub fn pick_builtins(detected: &[Detection]) -> Result<Vec<&'static BuiltinMeta>> {
    let detected_names: std::collections::HashSet<&str> =
        detected.iter().map(|d| d.builtin.name).collect();

    // Build options with detected items pre-selected
    let mut options: Vec<(&'static BuiltinMeta, bool)> = Vec::new();

    // Add detected builtins first (pre-selected)
    for detection in detected {
        options.push((detection.builtin, true));
    }

    // Add remaining builtins grouped by category
    for meta in BUILTINS_META {
        if !detected_names.contains(meta.name) {
            options.push((meta, false));
        }
    }

    // Create demand multi-select with options
    let mut ms = demand::MultiSelect::new("Select linters")
        .description("Space to toggle, Enter to confirm")
        .filterable(true);

    for (meta, selected) in &options {
        let label = format!("{}/{}", meta.category, meta.name);
        let opt = DemandOption::new(meta.name)
            .label(&label)
            .description(meta.description)
            .selected(*selected);
        ms = ms.option(opt);
    }

    let selected_names: Vec<&str> = ms.run()?;

    // Map selected names back to BuiltinMeta
    let result: Vec<&'static BuiltinMeta> = options
        .iter()
        .filter(|(meta, _)| selected_names.contains(&meta.name))
        .map(|(meta, _)| *meta)
        .collect();

    Ok(result)
}

/// Let user select which hooks to configure
pub fn pick_hooks() -> Result<Vec<String>> {
    let hooks = vec![
        ("pre-commit", "Run linters before committing"),
        ("check", "Manual check command (hk check)"),
        ("fix", "Manual fix command (hk fix)"),
        ("pre-push", "Run linters before pushing"),
    ];

    let mut ms = demand::MultiSelect::new("Select hooks to configure")
        .description("Space to toggle, Enter to confirm");

    for (name, desc) in &hooks {
        let opt = DemandOption::new(*name)
            .description(desc)
            .selected(DEFAULT_HOOKS.contains(name));
        ms = ms.option(opt);
    }

    let selected_names: Vec<&str> = ms.run()?;

    let result: Vec<String> = selected_names.iter().map(|s| s.to_string()).collect();

    Ok(result)
}
