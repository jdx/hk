use crate::Result;

pub mod pre_commit;

/// Migrate from other hook managers to hk
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Migrate {
    #[usage(subcommand)]
    command: MigrateCommands,
}

#[derive(Debug, usage_rs::Subcommands)]
enum MigrateCommands {
    /// Migrate from pre-commit (or prek) to hk
    PreCommit(pre_commit::PreCommit),
}

impl Migrate {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            MigrateCommands::PreCommit(cmd) => cmd.run().await,
        }
    }
}
