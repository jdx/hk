use crate::Result;

mod pre_commit;

#[derive(Debug, clap::Args)]
#[clap(visible_alias = "r", verbatim_doc_comment)]
pub struct Run {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    PreCommit(pre_commit::PreCommit),
}

impl Run {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::PreCommit(cmd) => cmd.run().await,
        }
    }
}
