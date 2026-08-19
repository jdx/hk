use crate::{Result, hook_options::HookOptions};

/// Run the pre-commit hook
#[derive(usage_rs::Args)]
pub struct PreCommit {
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

impl PreCommit {
    pub async fn run(mut self) -> Result<()> {
        // pre-commit receives no arguments from git
        self.hook.tctx.insert("hook_args", "");
        self.hook.run("pre-commit").await
    }
}
