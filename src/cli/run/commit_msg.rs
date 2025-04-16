use std::path::PathBuf;

use crate::Result;
use crate::hook_options::HookOptions;

#[derive(clap::Args)]
#[clap(visible_alias = "cm")]
pub struct CommitMsg {
    /// The path to the file that contains the commit message
    commit_msg_file: PathBuf,
    #[clap(flatten)]
    hook: HookOptions,
}

impl CommitMsg {
    pub async fn run(mut self) -> Result<()> {
        self.hook
            .tctx
            .insert("commit_msg_file", &self.commit_msg_file.to_string_lossy());
        self.hook.run("commit-msg").await
    }
}
