use std::path::PathBuf;

use crate::Result;

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap()]
pub struct Install {}

impl Install {
    pub async fn run(&self) -> Result<()> {
        let hooks = PathBuf::from(".git/hooks");
        let hook_file = hooks.join("pre-commit");
        let hook_content = r#"#!/bin/sh
hk run pre-commit "$@"
"#;
        xx::file::write(&hook_file, hook_content)?;
        xx::file::make_executable(&hook_file)?;
        println!("Installed hk hook: .git/hooks/pre-commit");
        Ok(())
    }
}
