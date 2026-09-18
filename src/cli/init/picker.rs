use demand::DemandOption;

use crate::Result;
use crate::builtins::{BUILTINS_META, BuiltinMeta};

use super::DEFAULT_HOOKS;

/// Select the repository-wide setup philosophy.
pub fn pick_preset() -> Result<String> {
    let select = demand::Select::new("How should hk choose tools?")
        .description("Existing project configuration always wins")
        .option(DemandOption::new("fast").label("Fast, simple defaults — prefer Biome"))
        .option(
            DemandOption::new("ecosystem")
                .label("Established ecosystem — prefer ESLint + Prettier"),
        )
        .option(DemandOption::new("custom").label("Customize — select linters yourself"));
    Ok(select.run()?.to_string())
}

/// Confirm a preset recommendation or switch to the individual tool picker.
pub fn confirm_preset() -> Result<bool> {
    let select = demand::Select::new("Accept this setup?")
        .description("Choose Customize to edit the recommended tools")
        .option(DemandOption::new("accept").label("Accept recommendation"))
        .option(DemandOption::new("customize").label("Customize tools"));
    Ok(select.run()? == "accept")
}

use super::detector::Detection;

/// Let user select builtins interactively.
pub fn pick_builtins(detected: &[Detection]) -> Result<Vec<&'static BuiltinMeta>> {
    pick_builtins_with_selected(&detected.iter().map(|d| d.builtin).collect::<Vec<_>>())
}

/// Build unique picker options, with recommendations selected first.
fn build_options(recommended: &[&'static BuiltinMeta]) -> Vec<(&'static BuiltinMeta, bool)> {
    let mut options: Vec<(&'static BuiltinMeta, bool)> = Vec::new();
    for meta in recommended {
        if !options
            .iter()
            .any(|(existing, _)| existing.name == meta.name)
        {
            options.push((*meta, true));
        }
    }
    for meta in BUILTINS_META {
        if !options
            .iter()
            .any(|(existing, _)| existing.name == meta.name)
        {
            options.push((meta, false));
        }
    }
    options
}

/// Pick tools while preselecting a preset recommendation.
pub fn pick_builtins_with_selected(
    recommended: &[&'static BuiltinMeta],
) -> Result<Vec<&'static BuiltinMeta>> {
    let options = build_options(recommended);
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
    Ok(options
        .iter()
        .filter(|(meta, _)| selected_names.contains(&meta.name))
        .map(|(meta, _)| *meta)
        .collect())
}

/// Let user select which hooks to configure
pub fn pick_hooks() -> Result<Vec<String>> {
    let hooks = vec![
        ("pre-commit", "Run linters before committing"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::BUILTINS_META;

    #[test]
    fn options_are_unique_and_recommendation_wins() {
        let biome = BUILTINS_META.iter().find(|m| m.name == "biome").unwrap();
        let options = build_options(&[biome, biome]);
        assert_eq!(options.iter().filter(|(m, _)| m.name == "biome").count(), 1);
        assert_eq!(
            options.iter().filter(|(m, _)| m.name == "prettier").count(),
            1
        );
        assert!(options.iter().find(|(m, _)| m.name == "biome").unwrap().1);
        assert!(
            !options
                .iter()
                .find(|(m, _)| m.name == "prettier")
                .unwrap()
                .1
        );
    }
}
