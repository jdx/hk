use std::path::PathBuf;

use crate::Result;
use crate::hook_options::HookOptions;

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
        self.hook
            .tctx
            .insert("commit_msg_file", &self.commit_msg_file.to_string_lossy());
        self.hook.tctx.insert("source", &self.source);
        self.hook.tctx.insert("sha", &self.sha.as_ref());
        self.hook.run("prepare-commit-msg").await
    }
}
