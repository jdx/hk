use crate::{Result, config::Config, env, git_util};
use eyre::bail;
use log::warn;
use std::process::Command;

/// Hook events installed when `--global` is used and no project `hk.pkl` is
/// available to enumerate them. Kept to the commonly-useful client-side hooks.
const DEFAULT_GLOBAL_EVENTS: &[&str] = &[
    "commit-msg",
    "post-checkout",
    "post-commit",
    "post-merge",
    "post-rewrite",
    "pre-commit",
    "pre-push",
    "pre-rebase",
    "prepare-commit-msg",
];

/// Sets up git hooks to run hk.
///
/// On Git 2.54+ this uses config-based hooks (`hook.<name>.command`), which
/// keeps `.git/hooks/` untouched and composes cleanly with other hook
/// managers. On older Git it falls back to writing script shims.
///
/// With `--global`, hooks are installed into the user's `~/.gitconfig` so
/// every repository picks them up without a per-repo install. In a project
/// without an `hk.pkl`, the installed hook exits silently — no-op.
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "i")]
pub struct Install {
    /// Install at user level (~/.gitconfig) so every repo on this machine
    /// gets hk hooks. Requires Git 2.54 or newer. In repos without an
    /// `hk.pkl`, the installed hook is a silent no-op.
    #[clap(long, verbatim_doc_comment)]
    global: bool,

    /// Force using the legacy `.git/hooks/` script shims instead of Git
    /// 2.54+ config-based hooks. Not compatible with `--global`.
    #[clap(long, verbatim_doc_comment, conflicts_with = "global")]
    legacy: bool,

    /// Use `mise x` to execute hooks. With this, it won't
    /// be necessary to activate mise in order to run hooks
    /// with mise tools.
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
    mise: bool,
}

impl Install {
    pub async fn run(&self) -> Result<()> {
        let command = if *env::HK_MISE || self.mise {
            "mise x -- hk"
        } else {
            "hk"
        };

        if self.global {
            if !git_util::git_at_least(2, 54) {
                bail!(
                    "`hk install --global` requires Git 2.54+ (config-based hooks). Detected git version does not support this. Upgrade git, or install per-repo with `hk install`."
                );
            }
            return install_global(command);
        }

        let use_config_hooks = !self.legacy && git_util::git_at_least(2, 54);

        // Load and validate the project config before touching anything, so a
        // broken `hk.pkl` doesn't leave the repo with its prior hooks removed.
        let config = Config::get()?;
        let events: Vec<String> = config
            .hooks
            .keys()
            .filter(|h| h.as_str() != "check" && h.as_str() != "fix")
            .cloned()
            .collect();

        // Clean up any prior installation so modes don't accumulate.
        let removed = remove_local_shims()? + remove_local_config_entries()?;

        if events.is_empty() {
            if removed > 0 {
                warn!(
                    "no hooks configured in hk.pkl — removed {removed} previously-installed hk hook(s) and did not install any new ones"
                );
            } else {
                warn!("no hooks configured in hk.pkl — nothing to install");
            }
            return Ok(());
        }

        if use_config_hooks {
            let result = install_local_config(&events, command);
            warn_if_global_overlap(&events);
            result
        } else {
            install_local_shims(&events, command)
        }
    }
}

/// Git aggregates `hook.<name>.command` values across scopes, so a local
/// install on top of a global one fires hk twice per event. Warn the user
/// and point them at the `enabled = false` escape hatch.
fn warn_if_global_overlap(events: &[String]) {
    let mut overlapping: Vec<&str> = Vec::new();
    for event in events {
        let key = format!("hook.hk-{event}.command");
        if let Ok(output) = Command::new("git")
            .args(["config", "--global", "--get", key.as_str()])
            .output()
            && output.status.success()
            && !output.stdout.is_empty()
        {
            overlapping.push(event);
        }
    }
    if overlapping.is_empty() {
        return;
    }
    warn!(
        "both global (~/.gitconfig) and local hk hooks are active for: {}. Git will run hk twice per event. To run only the local install, disable the global entries in this repo: {}",
        overlapping.join(", "),
        overlapping
            .iter()
            .map(|e| format!("`git config --local hook.hk-{e}.enabled false`"))
            .collect::<Vec<_>>()
            .join(" ; ")
    );
}

fn install_global(command: &str) -> Result<()> {
    remove_config_entries("--global")?;
    for event in DEFAULT_GLOBAL_EVENTS {
        write_config_hook("--global", command, event)?;
    }
    println!(
        "Installed hk global hooks in ~/.gitconfig for: {}",
        DEFAULT_GLOBAL_EVENTS.join(", ")
    );
    println!(
        "In repos without an hk.pkl, hk exits silently — add one with `hk init` to enable hooks."
    );
    Ok(())
}

fn install_local_config(events: &[String], command: &str) -> Result<()> {
    for event in events {
        write_config_hook("--local", command, event)?;
        println!("Installed hk hook via git config: hook.hk-{event}.command");
    }
    Ok(())
}

fn install_local_shims(events: &[String], command: &str) -> Result<()> {
    let git_path = git_util::find_git_path()?;
    let hooks = match git_util::worktree_hooks_path() {
        Some(path) => {
            xx::file::mkdirp(&path)?;
            path
        }
        None => {
            check_hooks_path_config()?;
            git_util::resolve_git_hooks_dir(&git_path)?
        }
    };
    for event in events {
        let hook_file = hooks.join(event);
        xx::file::write(&hook_file, git_hook_content(command, event))?;
        xx::file::make_executable(&hook_file)?;
        println!("Installed hk hook: {}", hook_file.display());
    }
    Ok(())
}

/// Write both `hook.hk-<event>.command` and `hook.hk-<event>.event` at the
/// given scope (`--local` or `--global`).
fn write_config_hook(scope: &str, command: &str, event: &str) -> Result<()> {
    let name = format!("hk-{event}");
    let cmd_key = format!("hook.{name}.command");
    let event_key = format!("hook.{name}.event");
    // Mirror the shim's HK=0 escape hatch so users can still disable hooks
    // with `HK=0 git commit` under config-based hooks.
    let cmd_value = format!(r#"test "${{HK:-1}}" = "0" || {command} run {event} --from-hook "$@""#);

    run_git(&["config", scope, cmd_key.as_str(), cmd_value.as_str()])?;
    // .event is multi-valued; replace-all keeps re-install idempotent.
    run_git(&["config", scope, "--replace-all", event_key.as_str(), event])?;
    Ok(())
}

fn remove_local_config_entries() -> Result<usize> {
    remove_config_entries("--local")
}

pub(crate) fn remove_config_entries(scope: &str) -> Result<usize> {
    let output = Command::new("git")
        .args([
            "config",
            scope,
            "--name-only",
            "--get-regexp",
            "^hook\\.hk-",
        ])
        .output()?;
    // git config --get-regexp: 0 = matches, 1 = no matches, ≥2 = real error
    // (e.g. unreadable config). Don't conflate "nothing to remove" with a
    // failed uninstall.
    let code = output.status.code().unwrap_or(1);
    if code == 1 {
        return Ok(0);
    }
    if !output.status.success() {
        bail!(
            "git config --get-regexp failed (exit {}): {}",
            code,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let keys: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    // Dedupe since multi-valued keys appear once per value.
    let mut seen = std::collections::BTreeSet::new();
    let mut removed = 0;
    for key in keys {
        if seen.insert(key.clone()) {
            run_git(&["config", scope, "--unset-all", key.as_str()])?;
            // Count one per hook event, not one per key (command + event).
            if key.ends_with(".command") {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

pub(crate) fn remove_local_shims() -> Result<usize> {
    let git_path = match git_util::find_git_path() {
        Ok(p) => p,
        Err(_) => return Ok(0),
    };
    let hooks = match git_util::worktree_hooks_path() {
        Some(path) => path,
        None => git_util::resolve_git_hooks_dir(&git_path)?,
    };
    if !hooks.is_dir() {
        return Ok(0);
    }
    let mut removed = 0;
    for p in xx::file::ls(&hooks)? {
        let content = match xx::file::read_to_string(&p) {
            Ok(content) => content,
            Err(_) => continue,
        };
        // Match the HK=0 guard that every hk-written shim has. This is more
        // specific than `hk run` alone, which could appear in an unrelated
        // user-written hook.
        if content.contains(r#"test "${HK:-1}" = "0""#) && content.contains("hk run") {
            xx::file::remove_file(&p)?;
            info!("removed hook: {}", xx::file::display_path(&p));
            removed += 1;
        }
    }
    Ok(removed)
}

fn run_git(args: &[&str]) -> Result<()> {
    let status = Command::new("git").args(args).status()?;
    if !status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(())
}

fn git_hook_content(hk: &str, hook: &str) -> String {
    format!(
        r#"#!/bin/sh
test "${{HK:-1}}" = "0" || exec {hk} run {hook} --from-hook "$@"
"#
    )
}

fn check_hooks_path_config() -> Result<()> {
    let check_config = |scope: &str| -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["config", scope, "--get", "core.hooksPath"])
            .output()?;

        if output.status.success() {
            let value = String::from_utf8(output.stdout)?.trim().to_string();
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
        Ok(None)
    };

    let mut warnings = Vec::new();

    if let Ok(Some(path)) = check_config("--global") {
        warnings.push(format!(
            "core.hooksPath is set globally to '{}'. This may prevent hk hooks from running.",
            path
        ));
        warnings
            .push("Run 'git config --global --unset-all core.hooksPath' to remove it.".to_string());
    }

    if let Ok(Some(path)) = check_config("--local") {
        warnings.push(format!(
            "core.hooksPath is set locally to '{}'. This may prevent hk hooks from running.",
            path
        ));
        warnings
            .push("Run 'git config --local --unset-all core.hooksPath' to remove it.".to_string());
    }

    for warning in &warnings {
        warn!("{}", warning);
    }

    Ok(())
}
