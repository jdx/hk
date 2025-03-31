use crate::Result;
use crate::config::Config;

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
        if config.hooks.get("fix").is_none() {
            eyre::bail!("fix hook not found in hk.pkl");
        }
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
