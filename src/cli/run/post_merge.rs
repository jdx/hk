use crate::Result;
use crate::hook_options::HookOptions;

/// Run the post-merge hook
#[derive(usage_rs::Args)]
pub struct PostMerge {
    /// Flag indicating whether the merge was a squash merge (1) or not (0)
    is_squash: String,
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

impl PostMerge {
    pub async fn run(mut self) -> Result<()> {
        self.hook.tctx.insert("hook_args", &self.is_squash);
        self.hook.run("post-merge").await
    }
}
