use std::path::PathBuf;
use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::{Result, git::Git};
use crate::{config::Config, step::CheckType};
use crate::{
    env,
    step::{RunType, Step},
};

#[derive(Debug, clap::Args)]
#[clap(visible_alias = "pcm")]
pub struct PrepareCommitMsg {
    /// The path to the file that contains the commit message so far
    commit_msg_file: PathBuf,
    /// The source of the commit message (e.g., "message", "template", "merge")
    source: String,
    /// The SHA of the commit being amended (if applicable)
    sha: Option<String>,
}

impl PrepareCommitMsg {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        if env::HK_SKIP_HOOK.contains("prepare-commit-msg") {
            warn!("prepare-commit-msg: skipping hook due to HK_SKIP_HOOK");
            return Ok(());
        }
        let repo = Git::new()?;
        static HOOK: LazyLock<IndexMap<String, Step>> = LazyLock::new(Default::default);
        let hook = config.hooks.get("prepare-commit-msg").unwrap_or(&HOOK);
        config
            .run_hook(
                false,
                hook,
                RunType::Check(CheckType::Check),
                &repo,
                &[],
                None,
                None,
            )
            .await?;
        Ok(())
    }
}
