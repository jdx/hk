use crate::{Result, config::Config, env, git_util};
use log::warn;
use std::process::Command;

/// Sets up git hooks to run hk
///
/// In a git worktree with a per-worktree core.hooksPath configured,
/// hooks are installed to that worktree-local directory. Otherwise
/// hooks go to the shared hooks directory.
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "i")]
pub struct Install {
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
        let config = Config::get()?;
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

        let command = if *env::HK_MISE || self.mise {
            "mise x -- hk".to_string()
        } else {
            "hk".to_string()
        };
        for hook in config.hooks.keys() {
            if hook == "check" || hook == "fix" {
                continue;
            }
            let hook_file = hooks.join(hook);
            xx::file::write(&hook_file, git_hook_content(&command, hook))?;
            xx::file::make_executable(&hook_file)?;
            println!("Installed hk hook: {}", hook_file.display());
        }
        Ok(())
    }
}

fn git_hook_content(hk: &str, hook: &str) -> String {
    format!(
        r#"#!/bin/sh
test "${{HK:-1}}" = "0" || exec {hk} run {hook} "$@"
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
