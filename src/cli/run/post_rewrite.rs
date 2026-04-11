use crate::Result;
use crate::hook_options::HookOptions;

#[derive(clap::Args)]
pub struct PostRewrite {
    /// The command that triggered the rewrite ("amend" or "rebase")
    command: String,
    #[clap(flatten)]
    hook: HookOptions,
}

impl PostRewrite {
    pub async fn run(mut self) -> Result<()> {
        self.hook.tctx.insert("hook_args", &self.command);
        self.hook.run("post-rewrite").await
    }
}
