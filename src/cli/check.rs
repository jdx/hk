use crate::Result;
use crate::config::Config;

/// Fixes code
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "c")]
pub struct Check {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Run on specific linter(s)
    #[clap(long)]
    linter: Vec<String>,
    /// Force stashing even if it's disabled via HK_STASH
    #[clap(long)]
    stash: bool,
    /// Start reference for checking files (requires --to-ref)
    #[clap(long)]
    from_ref: Option<String>,
    /// End reference for checking files (requires --from-ref)
    #[clap(long)]
    to_ref: Option<String>,
}

impl Check {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        if config.hooks.get("check").is_none() {
            eyre::bail!("check hook not found in hk.pkl");
        }
        config
            .run_hook(
                self.all,
                "check",
                &self.linter,
                Default::default(),
                self.from_ref.as_deref(),
                self.to_ref.as_deref(),
            )
            .await
    }
}
