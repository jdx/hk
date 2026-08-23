use crate::Result;

mod clear;

/// Manage hk internal cache
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Cache {
    #[usage(subcommand)]
    command: Commands,
}

#[derive(Debug, usage_rs::Subcommands)]
enum Commands {
    /// Clear the cache directory
    Clear(clear::Clear),
}

impl Cache {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::Clear(cmd) => cmd.run().await,
        }
    }
}
