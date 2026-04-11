use crate::Result;
use crate::hook_options::HookOptions;

#[derive(clap::Args)]
pub struct PostCheckout {
    /// SHA of the HEAD before the checkout
    prev_head: String,
    /// SHA of the new HEAD
    new_head: String,
    /// Flag indicating whether the checkout was a branch checkout (1) or file checkout (0)
    is_branch_checkout: String,
    #[clap(flatten)]
    hook: HookOptions,
}

impl PostCheckout {
    pub async fn run(mut self) -> Result<()> {
        self.hook.tctx.insert(
            "hook_args",
            &format!(
                "{} {} {}",
                self.prev_head, self.new_head, self.is_branch_checkout
            ),
        );
        self.hook.run("post-checkout").await
    }
}
