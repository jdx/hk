use crate::Result;
use crate::hook_options::HookOptions;

/// Run the post-commit hook
#[derive(usage_rs::Args)]
pub struct PostCommit {
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

impl PostCommit {
    pub async fn run(mut self) -> Result<()> {
        self.hook.insert_hook_var("hook_args", "");
        self.hook.run("post-commit").await
    }
}
