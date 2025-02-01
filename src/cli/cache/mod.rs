use crate::Result;

mod clear;

#[derive(Debug, clap::Args)]
pub struct Cache {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, clap::Subcommand)]
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
