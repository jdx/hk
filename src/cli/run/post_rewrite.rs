use std::io::IsTerminal;
use std::io::Read;

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
        let hook_stdin = if std::io::stdin().is_terminal() {
            String::new()
        } else {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            input
        };
        self.hook.tctx.insert("hook_stdin", &hook_stdin);
        self.hook.run("post-rewrite").await
    }
}
