use std::iter::once;
use std::sync::LazyLock;

use crate::config::Hook;
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
        config
            .run_hook(
                self.all,
                "fix",
                &self.linter,
                Default::default(),
                self.from_ref.as_deref(),
                self.to_ref.as_deref(),
            )
            .await
    }
}
