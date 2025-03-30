use std::path::PathBuf;
use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::config::Hook;
use crate::{Result, git::Git, tera::Context};
use crate::{config::Config, step::CheckType};
use crate::{
    env,
    step::{RunType, Step},
};

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
