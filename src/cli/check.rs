use std::iter::once;

use crate::{
    git::Git,
    step::{RunType, Step},
    Result,
};

use crate::config::Config;

/// Fixes code
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "c")]
pub struct Check {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Force stashing even if it's disabled via HK_STASH
    #[clap(long)]
    stash: bool,
}

impl Check {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        let repo = Git::new()?; // TODO: remove repo
        let hook = once(("check".to_string(), Step::check())).collect();
        config
            .run_hook(self.all, &hook, RunType::Check, &repo)
            .await
    }
}
