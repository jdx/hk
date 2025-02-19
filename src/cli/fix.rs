use std::iter::once;

use crate::step::RunType;
use crate::{config::Config, step::Step};
use crate::{git::Git, Result};

/// Fixes code
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "f")]
pub struct Fix {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
}

impl Fix {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        let repo = Git::new()?; // TODO: remove repo
        let hook = once(("fix".to_string(), Step::fix())).collect();
        config.run_hook(self.all, &hook, RunType::Fix, &repo).await
    }
}
