use crate::Result;
use crate::hook_options::HookOptions;

mod commit_msg;
mod post_checkout;
mod pre_commit;
mod pre_push;
mod prepare_commit_msg;

/// Run a hook
#[derive(clap::Args)]
#[clap(
    arg_required_else_help = true,
    visible_alias = "r",
    verbatim_doc_comment
)]
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
    PostCheckout(post_checkout::PostCheckout),
    PreCommit(pre_commit::PreCommit),
    PrePush(pre_push::PrePush),
    PrepareCommitMsg(prepare_commit_msg::PrepareCommitMsg),
}

impl Run {
    pub async fn run(mut self) -> Result<()> {
        if let Some(hook) = &self.other {
            // Hooks without a dedicated handler get an empty hook_args;
            // dedicated handlers insert the actual args via clap-parsed fields
            self.hook.tctx.insert("hook_args", "");
            return self.hook.run(hook).await;
        }
        if let Some(cmd) = self.command {
            return match cmd {
                Commands::CommitMsg(cmd) => cmd.run().await,
                Commands::PostCheckout(cmd) => cmd.run().await,
                Commands::PreCommit(cmd) => cmd.run().await,
                Commands::PrePush(cmd) => cmd.run().await,
                Commands::PrepareCommitMsg(cmd) => cmd.run().await,
            };
        }
        Ok(())
    }
}
