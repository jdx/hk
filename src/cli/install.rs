use std::path::PathBuf;

use crate::{Result, config::Config, env};

/// Sets up git hooks to run hk
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
        let hooks = PathBuf::from(".git/hooks");
        let add_hook = |hook: &str| {
            let hook_file = hooks.join(hook);
            let command = if *env::HK_MISE || self.mise {
                "mise x -- hk".to_string()
            } else {
                "hk".to_string()
            };
            xx::file::write(&hook_file, git_hook_content(&command, hook))?;
            #[cfg(unix)]
            xx::file::make_executable(&hook_file)?;
            println!("Installed hk hook: .git/hooks/{hook}");
            Result::<(), eyre::Report>::Ok(())
        };
        for hook in config.hooks.keys() {
            if hook == "check" || hook == "fix" {
                continue;
            }
            add_hook(hook)?;
        }
        Ok(())
    }
}

fn git_hook_content(hk: &str, hook: &str) -> String {
    if cfg!(windows) {
        // Windows batch file
        format!(
            r#"@echo off
if "%HK%"=="0" (
    exit /b 0
) else (
    {hk} run {hook} %*
)
"#
        )
    } else {
        // Unix shell script
        format!(
            r#"#!/bin/sh
test "${{HK:-1}}" = "0" || exec {hk} run {hook} "$@"
"#
        )
    }
}
