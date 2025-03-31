use crate::{Result, config::Config};

mod commit_msg;
mod pre_commit;
mod pre_push;
mod prepare_commit_msg;

/// Run a hook
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "r", verbatim_doc_comment)]
pub struct Run {
    #[clap(subcommand)]
    command: Option<Commands>,
    #[clap(hide = true)]
    other: Option<String>,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    CommitMsg(commit_msg::CommitMsg),
    PreCommit(pre_commit::PreCommit),
    PrePush(pre_push::PrePush),
    PrepareCommitMsg(prepare_commit_msg::PrepareCommitMsg),
}

impl Run {
    pub async fn run(self) -> Result<()> {
        if let Some(hook) = &self.other {
            return self.other(hook).await;
        }
        match self.command.unwrap() {
            Commands::CommitMsg(cmd) => cmd.run().await,
            Commands::PreCommit(cmd) => cmd.run().await,
            Commands::PrePush(cmd) => cmd.run().await,
            Commands::PrepareCommitMsg(cmd) => cmd.run().await,
        }
    }

    async fn other(&self, hook: &str) -> Result<()> {
        let config = Config::get()?;
        config
            .run_hook(true, hook, &[], Default::default(), None, None)
            .await
    }
}
