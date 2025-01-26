use std::path::PathBuf;

use crate::Result;

/// Sets up git hooks to run angler
#[derive(Debug, clap::Args)]
#[clap()]
pub struct Init {}

impl Init {
    pub async fn run(&self) -> Result<()> {
        let angler_file = PathBuf::from("angler.toml");
        let hook_content = r#"
[[pre-commit]]
run = "prettier --check ."
"#;
        xx::file::write(angler_file, hook_content.trim_start())?;
        Ok(())
    }
}
