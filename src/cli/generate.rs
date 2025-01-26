use crate::Result;
use std::path::PathBuf;

/// Generate a default hk.toml configuration file
#[derive(Debug, clap::Args)]
#[clap()]
pub struct Generate {}

impl Generate {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.toml");
        if hk_file.exists() {
            println!("hk.toml already exists");
            return Ok(());
        }

        let config_content = r#"[[pre-commit]]
plugin = "end-of-file-fixer"
"#;
        xx::file::write(hk_file, config_content)?;
        println!("Created hk.toml");
        Ok(())
    }
} 
