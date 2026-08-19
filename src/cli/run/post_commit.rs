use crate::Result;
use crate::hook_options::HookOptions;

#[derive(usage_derive::Args)]
pub struct PostCommit {
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

impl PostCommit {
    pub async fn run(mut self) -> Result<()> {
        self.hook.tctx.insert("hook_args", "");
        self.hook.run("post-commit").await
    }
}
