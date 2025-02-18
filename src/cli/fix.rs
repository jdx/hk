use crate::config::Config;
use crate::step::RunType;
use crate::{git::Git, Result};

/// Fixes code
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "f")]
pub struct Fix {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Force stashing even if it's disabled via HK_STASH
    #[clap(long)]
    stash: bool,
}

impl Fix {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        let mut repo = Git::new()?;
        let run_type = if self.all {
            RunType::FixAll
        } else {
            RunType::Fix
        };
        if !self.all {
            repo.stash_unstaged(self.stash)?;
        }
        let mut result = if let Some(hook) = &config.pre_commit {
            config.run_hook(hook, run_type, &repo).await
        } else {
            Ok(())
        };

        if let Err(err) = repo.pop_stash() {
            if result.is_ok() {
                result = Err(err);
            } else {
                warn!("Failed to pop stash: {}", err);
            }
        }
        result
    }
}
