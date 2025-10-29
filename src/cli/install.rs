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
        let cwd = std::env::current_dir()?;
        let git_dir = xx::file::find_up(&cwd, &[".git"]).ok_or_else(|| {
            eyre::eyre!("No .git directory found in this or any parent directory")
        })?;
        let hooks = git_dir.join("hooks");
        let add_hook = |hook: &str| {
            let hook_file = hooks.join(hook);
            let command = if *env::HK_MISE || self.mise {
                "mise x -- hk".to_string()
            } else {
                "hk".to_string()
            };
            xx::file::write(&hook_file, git_hook_content(&command, hook))?;
            xx::file::make_executable(&hook_file)?;
            println!("Installed hk hook: {}", hook_file.display());
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
    format!(
        r#"#!/bin/sh
test "${{HK:-1}}" = "0" || exec {hk} run {hook} "$@"
"#
    )
}
