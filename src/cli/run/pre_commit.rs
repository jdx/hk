use crate::Result;
use crate::config::Config;

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "co")]
pub struct PreCommit {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Run fix command instead of run command
    /// This is the default behavior unless HK_FIX=0
    #[clap(short, long, overrides_with = "check")]
    fix: bool,
    /// Run run command instead of fix command
    #[clap(short, long, overrides_with = "fix")]
    check: bool,
    /// Run on specific linter(s)
    #[clap(long)]
    linter: Vec<String>,
    /// Start reference for checking files (requires --to-ref)
    #[clap(long)]
    from_ref: Option<String>,
    /// End reference for checking files (requires --from-ref)
    #[clap(long)]
    to_ref: Option<String>,
}

impl PreCommit {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        config
            .run_hook(
                self.all,
                "pre-commit",
                &self.linter,
                Default::default(),
                self.from_ref.as_deref(),
                self.to_ref.as_deref(),
            )
            .await
    }
}
