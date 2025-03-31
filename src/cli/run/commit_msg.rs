use std::path::PathBuf;

use crate::config::Config;
use crate::{Result, tera::Context};

#[derive(Debug, clap::Args)]
#[clap(visible_alias = "cm")]
pub struct CommitMsg {
    /// The path to the file that contains the commit message
    commit_msg_file: PathBuf,
}

impl CommitMsg {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        let mut tctx = Context::default();
        tctx.insert("commit_msg_file", &self.commit_msg_file.to_string_lossy());
        config
            .run_hook(false, "commit-msg", &[], tctx, None, None)
            .await
    }
}
