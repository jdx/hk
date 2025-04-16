use crate::Result;
use crate::hook_options::HookOptions;

mod commit_msg;
mod pre_commit;
mod pre_push;
mod prepare_commit_msg;

/// Run a hook
#[derive(clap::Args)]
#[clap(visible_alias = "r", verbatim_doc_comment)]
pub struct Run {
    #[clap(subcommand)]
    command: Option<Commands>,
    #[clap(hide = true)]
    other: Option<String>,
    #[clap(flatten)]
    hook: HookOptions,
}

#[derive(clap::Subcommand)]
enum Commands {
    CommitMsg(commit_msg::CommitMsg),
    PreCommit(pre_commit::PreCommit),
    PrePush(pre_push::PrePush),
    PrepareCommitMsg(prepare_commit_msg::PrepareCommitMsg),
}

impl Run {
    pub async fn run(self) -> Result<()> {
        if let Some(hook) = &self.other {
            return self.hook.run(hook).await;
        }
        match self.command.unwrap() {
            Commands::CommitMsg(cmd) => cmd.run().await,
            Commands::PreCommit(cmd) => cmd.run().await,
            Commands::PrePush(cmd) => cmd.run().await,
            Commands::PrepareCommitMsg(cmd) => cmd.run().await,
        }
    }
}
