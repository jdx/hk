use std::io::IsTerminal;
use std::io::Read;

use crate::Result;
use crate::hook_options::HookOptions;

/// Run the post-rewrite hook
#[derive(usage_rs::Args)]
pub struct PostRewrite {
    /// The command that triggered the rewrite ("amend" or "rebase")
    command: String,
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

impl PostRewrite {
    pub async fn run(mut self) -> Result<()> {
        if self.hook.reads_file_list_from_stdin() {
            return Err(eyre::eyre!(
                "--files0-from - cannot be used with post-rewrite because the hook reads rewrite data from stdin"
            ));
        }
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
