use std::path::PathBuf;

use crate::Result;

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap()]
pub struct Init {}

impl Init {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.toml");
        let hook_content = r#"
[[pre-commit]]
run = "prettier --check ."
"#;
        xx::file::write(hk_file, hook_content.trim_start())?;
        Ok(())
    }
}
