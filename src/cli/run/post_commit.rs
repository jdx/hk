use crate::Result;
use crate::hook_options::HookOptions;

#[derive(clap::Args)]
pub struct PostCommit {
    #[clap(flatten)]
    hook: HookOptions,
}

impl PostCommit {
    pub async fn run(mut self) -> Result<()> {
        self.hook.tctx.insert("hook_args", "");
        self.hook.run("post-commit").await
    }
}
