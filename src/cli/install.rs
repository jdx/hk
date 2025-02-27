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
                format!("mise x -- hk run {hook}")
            } else {
                format!("hk run {hook}")
            };
            let hook_content = format!(
                r#"#!/bin/sh
if [ "$HK" = "0" ] || [ "$HK" = "false" ]; then
    exit 0
fi
exec {command} "$@"
"#
            );
            xx::file::write(&hook_file, &hook_content)?;
            xx::file::make_executable(&hook_file)?;
            println!("Installed hk hook: .git/hooks/{hook}");
            Result::<(), eyre::Report>::Ok(())
        };
        if config.hooks.contains_key("pre-commit") {
            add_hook("pre-commit")?;
        }
        if config.hooks.contains_key("pre-push") {
            add_hook("pre-push")?;
        }
        Ok(())
    }
}
