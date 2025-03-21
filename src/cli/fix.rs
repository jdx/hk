use std::iter::once;

use crate::step::RunType;
use crate::{Result, git::Git};
use crate::{config::Config, step::Step};

/// Fixes code
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "f")]
pub struct Fix {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Run on specific linter(s)
    #[clap(long)]
    linter: Vec<String>,
    /// Start reference for fixing files (requires --to-ref)
    #[clap(long)]
    from_ref: Option<String>,
    /// End reference for fixing files (requires --from-ref)
    #[clap(long)]
    to_ref: Option<String>,
}

impl Fix {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        let repo = Git::new()?; // TODO: remove repo
        let hook = once(("fix".to_string(), Step::fix())).collect();

        // Check if both from_ref and to_ref are provided or neither
        if (self.from_ref.is_some() && self.to_ref.is_none())
            || (self.from_ref.is_none() && self.to_ref.is_some())
        {
            return Err(eyre::eyre!(
                "Both --from-ref and --to-ref must be provided together"
            ));
        }

        config
            .run_hook(
                self.all,
                &hook,
                RunType::Fix,
                &repo,
                &self.linter,
                Default::default(),
                self.from_ref.as_deref(),
                self.to_ref.as_deref(),
            )
            .await
    }
}
