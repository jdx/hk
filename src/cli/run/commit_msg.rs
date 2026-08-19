use std::path::PathBuf;

use crate::Result;
use crate::git_util;
use crate::hook_options::HookOptions;

#[derive(usage_rs::Args)]
pub struct CommitMsg {
    /// The path to the file that contains the commit message
    commit_msg_file: PathBuf,
    #[usage(flatten)]
    pub(super) hook: HookOptions,
}

impl CommitMsg {
    pub async fn run(mut self) -> Result<()> {
        let resolved = git_util::resolve_git_relative_path(&self.commit_msg_file)?;
        self.hook
            .tctx
            .insert("commit_msg_file", &resolved.to_string_lossy());
        self.hook
            .tctx
            .insert("hook_args", &resolved.to_string_lossy());
        self.hook.run("commit-msg").await
    }
}
