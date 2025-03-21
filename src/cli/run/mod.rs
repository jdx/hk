use crate::Result;

mod pre_commit;
mod pre_push;
mod prepare_commit_msg;

/// Run a hook
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "r", verbatim_doc_comment)]
pub struct Run {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    PreCommit(pre_commit::PreCommit),
    PrePush(pre_push::PrePush),
    PrepareCommitMsg(prepare_commit_msg::PrepareCommitMsg),
}

impl Run {
    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::PreCommit(cmd) => cmd.run().await,
            Commands::PrePush(cmd) => cmd.run().await,
            Commands::PrepareCommitMsg(cmd) => cmd.run().await,
        }
    }
}
