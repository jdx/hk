use crate::Result;
use crate::hook_options::HookOptions;

mod commit_msg;
mod post_checkout;
mod post_commit;
mod post_merge;
mod post_rewrite;
mod pre_commit;
mod pre_push;
mod pre_rebase;
mod prepare_commit_msg;

/// Run a hook
#[derive(usage_rs::Args)]
#[command(arg_required_else_help)]
pub struct Run {
    #[usage(subcommand)]
    command: Option<Commands>,
    #[usage(arg, hide)]
    other: Option<String>,
    #[usage(flatten)]
    hook: HookOptions,
}

#[derive(usage_rs::Subcommands)]
enum Commands {
    #[usage(alias = "cm")]
    CommitMsg(commit_msg::CommitMsg),
    PostCheckout(post_checkout::PostCheckout),
    PostCommit(post_commit::PostCommit),
    PostMerge(post_merge::PostMerge),
    PostRewrite(post_rewrite::PostRewrite),
    #[usage(alias = "pc")]
    PreCommit(pre_commit::PreCommit),
    #[usage(alias = "pp")]
    PrePush(pre_push::PrePush),
    PreRebase(pre_rebase::PreRebase),
    #[usage(alias = "pcm")]
    PrepareCommitMsg(prepare_commit_msg::PrepareCommitMsg),
}

impl Run {
    pub(crate) fn output_format(&self) -> Option<crate::structured_output::OutputFormat> {
        let command_format = self.command.as_ref().and_then(|command| match command {
            Commands::CommitMsg(command) => command.hook.format,
            Commands::PostCheckout(command) => command.hook.format,
            Commands::PostCommit(command) => command.hook.format,
            Commands::PostMerge(command) => command.hook.format,
            Commands::PostRewrite(command) => command.hook.format,
            Commands::PreCommit(command) => command.hook.format,
            Commands::PrePush(command) => command.hook.format,
            Commands::PreRebase(command) => command.hook.format,
            Commands::PrepareCommitMsg(command) => command.hook.format,
        });
        command_format.or(self.hook.format)
    }

    pub async fn run(mut self) -> Result<()> {
        if let Some(hook) = &self.other {
            // Hooks without a dedicated handler get an empty hook_args;
            // dedicated handlers insert the actual args via clap-parsed fields
            self.hook.tctx.insert("hook_args", "");
            return self.hook.run(hook).await;
        }
        let cmd = self
            .command
            .expect("arg_required_else_help rejects a run command without a hook");
        match cmd {
            Commands::CommitMsg(cmd) => cmd.run().await,
            Commands::PostCheckout(cmd) => cmd.run().await,
            Commands::PostCommit(cmd) => cmd.run().await,
            Commands::PostMerge(cmd) => cmd.run().await,
            Commands::PostRewrite(cmd) => cmd.run().await,
            Commands::PreCommit(cmd) => cmd.run().await,
            Commands::PrePush(cmd) => cmd.run().await,
            Commands::PreRebase(cmd) => cmd.run().await,
            Commands::PrepareCommitMsg(cmd) => cmd.run().await,
        }
    }
}
