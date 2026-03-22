use std::path::PathBuf;

use crate::Result;
use crate::git_util;
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
        let resolved = git_util::resolve_git_relative_path(&self.commit_msg_file)?;
        self.hook
            .tctx
            .insert("commit_msg_file", &resolved.to_string_lossy());
        self.hook.run("commit-msg").await
    }
}
