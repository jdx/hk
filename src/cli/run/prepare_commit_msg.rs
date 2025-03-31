use std::path::PathBuf;

use crate::config::Config;
use crate::{Result, tera::Context};

#[derive(Debug, clap::Args)]
#[clap(visible_alias = "pcm")]
pub struct PrepareCommitMsg {
    /// The path to the file that contains the commit message so far
    commit_msg_file: PathBuf,
    /// The source of the commit message (e.g., "message", "template", "merge")
    source: Option<String>,
    /// The SHA of the commit being amended (if applicable)
    sha: Option<String>,
}

impl PrepareCommitMsg {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        let mut tctx = Context::default();
        tctx.insert("commit_msg_file", &self.commit_msg_file.to_string_lossy());
        tctx.insert("source", &self.source);
        tctx.insert("sha", &self.sha.as_ref());
        config
            .run_hook(false, "prepare-commit-msg", &[], tctx, None, None)
            .await
    }
}
