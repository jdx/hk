use crate::Result;
use crate::hook_options::HookOptions;

#[derive(clap::Args)]
pub struct PreRebase {
    /// The upstream from which the series was forked
    upstream: String,
    /// The branch being rebased (unset when rebasing the current branch)
    branch: Option<String>,
    #[clap(flatten)]
    hook: HookOptions,
}

impl PreRebase {
    pub async fn run(mut self) -> Result<()> {
        let args = match &self.branch {
            Some(b) => format!("{} {}", self.upstream, b),
            None => self.upstream.clone(),
        };
        self.hook.tctx.insert("hook_args", &args);
        self.hook.run("pre-rebase").await
    }
}
