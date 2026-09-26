use crate::Result;
use crate::hook_options::HookOptions;

/// Run the post-checkout hook
#[derive(usage_rs::Args)]
pub struct PostCheckout {
    /// SHA of the HEAD before the checkout
    prev_head: String,
    /// SHA of the new HEAD
    new_head: String,
    /// Flag indicating whether the checkout was a branch checkout (1) or file checkout (0)
    is_branch_checkout: String,
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

impl PostCheckout {
    pub async fn run(mut self) -> Result<()> {
        self.hook.insert_hook_var("prev_head", &self.prev_head);
        self.hook.insert_hook_var("new_head", &self.new_head);
        self.hook
            .insert_hook_var("is_branch_checkout", &(self.is_branch_checkout == "1"));
        self.hook.insert_hook_var(
            "hook_args",
            &format!(
                "{} {} {}",
                self.prev_head, self.new_head, self.is_branch_checkout
            ),
        );
        self.hook.run("post-checkout").await
    }
}
