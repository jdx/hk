use std::path::PathBuf;

use crate::Result;
use crate::hook_options::HookOptions;
use crate::git_util;

#[derive(clap::Args)]
#[clap(visible_alias = "pcm")]
pub struct PrepareCommitMsg {
    /// The path to the file that contains the commit message so far
    commit_msg_file: PathBuf,
    /// The source of the commit message (e.g., "message", "template", "merge")
    source: Option<String>,
    /// The SHA of the commit being amended (if applicable)
    sha: Option<String>,
    #[clap(flatten)]
    hook: HookOptions,
}

impl PrepareCommitMsg {
    pub async fn run(mut self) -> Result<()> {
        let resolved = git_util::resolve_git_relative_path(&self.commit_msg_file)?;
        self.hook
            .tctx
            .insert("commit_msg_file", &resolved.to_string_lossy());
        self.hook.tctx.insert("source", &self.source);
        self.hook.tctx.insert("sha", &self.sha.as_ref());
        self.hook.run("prepare-commit-msg").await
    }
}
