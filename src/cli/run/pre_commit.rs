use crate::{Result, hook_options::HookOptions};

/// Sets up git hooks to run hk
#[derive(clap::Args)]
#[clap(visible_alias = "pc")]
pub struct PreCommit {
    #[clap(flatten)]
    hook: HookOptions,
}

impl PreCommit {
    pub async fn run(self) -> Result<()> {
        self.hook.run("pre-commit").await
    }
}
