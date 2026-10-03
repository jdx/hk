//! Warnings for `hk validate`.
//!
//! These cover configs that load and run but probably do not do what their
//! author meant. They are never errors: a config that works today must keep
//! working.

use indexmap::{IndexMap, IndexSet};
use serde_json::Value;

use crate::{
    hook::{Hook, StepOrGroup},
    step::Pattern,
    step_group::StepGroup,
    suggest::did_you_mean_hint,
};

const CONFIG_KEYS: &[&str] = &[
    "min_hk_version",
    "steps",
    "hooks",
    "default_branch",
    "env",
    "fail_fast",
    "display_skip_reasons",
    "hide_warnings",
    "warnings",
    "exclude",
    "stage",
    "profiles",
    "skip_hooks",
    "skip_steps",
    "stash_backup_count",
    "subprojects",
    "terminal_progress",
    "walk_ignore",
];

const HOOK_KEYS: &[&str] = &[
    "name",
    "steps",
    "enabled",
    "fix",
    "stash",
    "stage",
    "fail_on_fix",
    "env",
    "report",
];

const GROUP_KEYS: &[&str] = &[
    "_type",
    "name",
    "workspace_indicator",
    "prefix",
    "dir",
    "shell",
    "stage",
    "exclude",
    "steps",
];

const STEP_KEYS: &[&str] = &[
    "_type",
    "category",
    "description",
    "name",
    "profiles",
    "glob",
    "types",
    "match_any",
    "interactive",
    "stdin",
    "required",
    "depends",
    "allow_failure",
    "shell",
    "check",
    "check_list_files",
    "check_diff",
    "apply_check_diff",
    "check_after_diff",
    "check_failed_files",
    "fix",
    "workspace_indicator",
    "prefix",
    "dir",
    "condition",
    "step_condition",
    "check_first",
    "batch",
    "batch_min_files",
    "stomp",
    "env",
    "stage",
    "exclude",
    "exclusive",
    "allow_binary",
    "allow_symlinks",
    "root",
    "hide",
    "tests",
    "output_summary",
    "diagnostic_format",
    "diagnostic_tool",
];

const SELECTOR_KEYS: &[&str] = &["glob", "types"];

/// Properties of the evaluated config that hk does not know. Debug builds
/// reject them while loading; release builds silently drop them, so a typo
/// such as `chek` just never runs.
pub fn unknown_properties(config: &Value) -> Vec<String> {
    let mut out = vec![];
    let Some(config) = config.as_object() else {
        return out;
    };
    check_keys(config, CONFIG_KEYS, "the top level of the config", &mut out);
    if let Some(steps) = config.get("steps").and_then(Value::as_object) {
        for (name, step) in steps {
            check_step_or_group(step, &format!("step '{name}'"), &mut out);
        }
    }
    if let Some(hooks) = config.get("hooks").and_then(Value::as_object) {
        for (hook_name, hook) in hooks {
            let Some(hook) = hook.as_object() else {
                continue;
            };
            check_keys(hook, HOOK_KEYS, &format!("hook '{hook_name}'"), &mut out);
            if let Some(steps) = hook.get("steps").and_then(Value::as_object) {
                for (name, step) in steps {
                    let location = format!("step '{name}' in hook '{hook_name}'");
                    check_step_or_group(step, &location, &mut out);
                }
            }
        }
    }
    out
}

fn check_keys(
    object: &serde_json::Map<String, Value>,
    known: &[&str],
    location: &str,
    out: &mut Vec<String>,
) {
    for key in object.keys() {
        if !known.contains(&key.as_str()) {
            let hint = did_you_mean_hint(key, known.iter().copied());
            out.push(format!(
                "unknown property '{key}' in {location}; hk ignores it.{hint}"
            ));
        }
    }
}

fn check_step_or_group(value: &Value, location: &str, out: &mut Vec<String>) {
    let Some(object) = value.as_object() else {
        return;
    };
    let is_group = match object.get("_type").and_then(Value::as_str) {
        Some("group") => true,
        Some(_) => object.contains_key("steps"),
        None => object.contains_key("steps"),
    };
    if !is_group {
        check_step(object, location, out);
        return;
    }
    check_keys(object, GROUP_KEYS, location, out);
    if let Some(steps) = object.get("steps").and_then(Value::as_object) {
        for (name, step) in steps {
            if let Some(step) = step.as_object() {
                check_step(step, &format!("step '{name}' in group {location}"), out);
            }
        }
    }
}

fn check_step(object: &serde_json::Map<String, Value>, location: &str, out: &mut Vec<String>) {
    check_keys(object, STEP_KEYS, location, out);
    if let Some(selectors) = object.get("match_any").and_then(Value::as_array) {
        for selector in selectors.iter().filter_map(Value::as_object) {
            check_keys(
                selector,
                SELECTOR_KEYS,
                &format!("a match_any selector of {location}"),
                out,
            );
        }
    }
}

/// Suspicious but loadable settings in a resolved config. `implicit` names the
/// hooks hk derived from top-level `steps`; those are linted once, as `steps`,
/// instead of once per derived hook.
pub fn lint_hooks(
    hooks: &IndexMap<String, Hook>,
    top_level_steps: &IndexMap<String, StepOrGroup>,
    implicit: &IndexSet<String>,
) -> Vec<String> {
    let mut out = IndexSet::new();
    if !top_level_steps.is_empty() {
        let synthetic = Hook {
            name: "steps".to_string(),
            steps: top_level_steps.clone(),
            ..Default::default()
        };
        lint_hook("steps", &synthetic, &mut out);
    }
    for (name, hook) in hooks {
        if !implicit.contains(name) {
            lint_hook(name, hook, &mut out);
        }
    }
    out.into_iter().collect()
}

fn lint_hook(hook_name: &str, hook: &Hook, out: &mut IndexSet<String>) {
    let group_names: IndexSet<&str> = hook
        .steps
        .iter()
        .filter(|(_, s)| matches!(s, StepOrGroup::Group(_)))
        .map(|(name, _)| name.as_str())
        .collect();
    let groups = StepGroup::build_all(hook.steps.values().cloned().collect());
    let mut group_of: IndexMap<&str, usize> = IndexMap::new();
    for (index, group) in groups.iter().enumerate() {
        for name in group.steps.keys() {
            group_of.insert(name, index);
        }
    }
    let where_ = |name: &str| format!("Step '{name}' in hook '{hook_name}'");
    for (index, group) in groups.iter().enumerate() {
        for (name, step) in &group.steps {
            for dep in &step.depends {
                if dep == name {
                    continue; // already an error
                }
                match group_of.get(dep.as_str()) {
                    Some(dep_index) if *dep_index > index => out.insert(format!(
                        "{} depends on '{dep}', which runs in a later group. `depends` only \
                        orders steps in the same group, so this has no effect.",
                        where_(name)
                    )),
                    Some(_) => false,
                    None if group_names.contains(dep.as_str()) => out.insert(format!(
                        "{} depends on '{dep}', which is a group. `depends` names steps; \
                        list the steps inside the group instead.",
                        where_(name)
                    )),
                    None => out.insert(format!(
                        "{} depends on unknown step '{dep}'.{}",
                        where_(name),
                        did_you_mean_hint(dep, group_of.keys().copied())
                    )),
                };
            }
            if step.check.is_none()
                && step.fix.is_none()
                && step.check_list_files.is_none()
                && step.check_diff.is_none()
            {
                out.insert(format!(
                    "{} has no check, fix, check_list_files or check_diff command, so it does nothing.",
                    where_(name)
                ));
            }
            let selectors = step.match_any.iter().flatten().map(|s| s.glob.as_ref());
            let patterns = [step.glob.as_ref(), step.exclude.as_ref()]
                .into_iter()
                .chain(selectors);
            for pattern in patterns.flatten() {
                if let Pattern::Globs(globs) = pattern {
                    for glob in globs {
                        if let Some(problem) = glob_problem(glob) {
                            out.insert(format!("{} has glob '{glob}': {problem}", where_(name)));
                        }
                    }
                }
            }
        }
    }
}

fn glob_problem(glob: &str) -> Option<&'static str> {
    if glob.starts_with("./") {
        return Some(
            "files are matched as repo-relative paths such as 'src/a.rs', so a leading './' never matches.",
        );
    }
    if glob.starts_with('/') {
        return Some(
            "files are matched as repo-relative paths, so a leading '/' never matches.",
        );
    }
    if glob.starts_with('!') {
        return Some(
            "'!' negation is not supported in globs; use `exclude` to leave files out.",
        );
    }
    let mut depth = 0usize;
    for c in glob.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                return Some(
                    "a comma outside braces is part of the file name; use a list of globs or '{a,b}' alternation.",
                );
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::Step;
    use serde_json::json;

    fn keys_of<T: serde::Serialize + Default>() -> Vec<String> {
        serde_json::to_value(T::default())
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    #[test]
    fn known_key_lists_cover_every_serialized_field() {
        let cases: [(&str, Vec<String>, &[&str]); 4] = [
            ("Config", keys_of::<crate::config::Config>(), CONFIG_KEYS),
            ("Hook", keys_of::<Hook>(), HOOK_KEYS),
            ("Step", keys_of::<Step>(), STEP_KEYS),
            ("StepGroup", keys_of::<StepGroup>(), GROUP_KEYS),
        ];
        for (what, serialized, known) in cases {
            for key in serialized {
                assert!(known.contains(&key.as_str()), "{what} key {key} missing");
            }
        }
    }

    #[test]
    fn finds_misspelled_properties_everywhere() {
        let config = json!({
            "hooks": {
                "check": {
                    "steps": {
                        "lint": { "chek": "true", "glob": "*.rs", "match_any": [{ "glb": "*" }] },
                        "grp": { "_type": "group", "stpes": {}, "steps": { "a": { "fxi": "x" } } }
                    },
                    "enbled": true
                }
            },
            "steps": { "top": { "dpends": [] } },
            "min_hk_versoin": "1"
        });
        let found = unknown_properties(&config).join("\n");
        for needle in [
            "unknown property 'chek' in step 'lint' in hook 'check'; hk ignores it. Did you mean 'check'?",
            "unknown property 'glb' in a match_any selector",
            "unknown property 'stpes' in step 'grp' in hook 'check'",
            "unknown property 'fxi' in step 'a' in group step 'grp' in hook 'check'",
            "unknown property 'enbled' in hook 'check'",
            "unknown property 'dpends' in step 'top'",
            "unknown property 'min_hk_versoin' in the top level",
        ] {
            assert!(found.contains(needle), "missing {needle:?} in:\n{found}");
        }
        assert!(unknown_properties(&json!({"hooks": {"check": {"steps": {}}}})).is_empty());
    }

    fn named(name: &str, depends: &[&str]) -> Step {
        Step {
            name: name.into(),
            depends: depends.iter().map(|d| d.to_string()).collect(),
            check: Some(crate::step::Command::Shell("true".into())),
            ..Default::default()
        }
    }

    fn lint(steps: Vec<Step>, groups: Vec<(&str, Vec<Step>)>) -> Vec<String> {
        let mut hook = Hook {
            name: "check".into(),
            ..Default::default()
        };
        for step in steps {
            hook.steps
                .insert(step.name.clone(), StepOrGroup::Step(Box::new(step)));
        }
        for (name, members) in groups {
            let group = StepGroup {
                name: Some(name.into()),
                steps: members.into_iter().map(|s| (s.name.clone(), s)).collect(),
                ..Default::default()
            };
            hook.steps
                .insert(name.into(), StepOrGroup::Group(Box::new(group)));
        }
        let hooks = IndexMap::from([("check".to_string(), hook)]);
        lint_hooks(&hooks, &IndexMap::new(), &IndexSet::new())
    }

    #[test]
    fn warns_about_dependencies_that_never_order_anything() {
        let mut later = named("later", &[]);
        later.exclusive = true;
        let mut first = named("first", &["later", "lter", "grp", "nope-at-all"]);
        first.exclusive = false;
        let out = lint(vec![first, later], vec![("grp", vec![named("inner", &[])])]).join("\n");
        assert!(out.contains("depends on 'later', which runs in a later group"), "{out}");
        assert!(out.contains("depends on unknown step 'lter'. Did you mean 'later'?"), "{out}");
        assert!(out.contains("depends on 'grp', which is a group"), "{out}");
        assert!(out.contains("unknown step 'nope-at-all'.") && !out.contains("nope-at-all'. Did"), "{out}");
    }

    #[test]
    fn warns_about_command_less_steps_and_bad_globs() {
        let mut empty = named("empty", &[]);
        empty.check = None;
        let mut globby = named("globby", &[]);
        globby.glob = Some(Pattern::Globs(vec![
            "./src/*.rs".into(),
            "/abs/*.rs".into(),
            "!vendor/**".into(),
            "*.js,*.ts".into(),
            "*.{js,ts}".into(),
        ]));
        let out = lint(vec![empty, globby], vec![]);
        let all = out.join("\n");
        assert!(all.contains("Step 'empty' in hook 'check' has no check"), "{all}");
        assert_eq!(out.iter().filter(|m| m.contains("has glob")).count(), 4, "{all}");
        assert!(!all.contains("'*.{js,ts}'"), "{all}");
    }

    #[test]
    fn clean_config_has_no_warnings() {
        assert!(lint(vec![named("a", &[]), named("b", &["a"])], vec![]).is_empty());
    }
}
