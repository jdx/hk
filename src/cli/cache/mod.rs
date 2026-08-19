use crate::Result;

mod clear;

/// Manage hk internal cache
#[derive(Debug, usage_derive::Args)]
#[usage(arg, hide)] // TODO: unhide if we actually use cache (which we probably will)
pub struct Cache {
    #[usage(subcommand)]
    command: Commands,
}

#[derive(Debug, usage_derive::Subcommands)]
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
