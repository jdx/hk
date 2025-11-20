use crate::{Result, config::Config, tera::Context};

#[derive(clap::Args)]
pub(crate) struct HookOptions {
    /// Run on specific files
    #[clap(conflicts_with_all = &["all", "fix", "check"], value_hint = clap::ValueHint::FilePath)]
    pub files: Option<Vec<String>>,
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    pub all: bool,
    /// Run check command instead of fix command
    #[clap(short, long, overrides_with = "fix")]
    pub check: bool,
    /// Exclude files that otherwise would have been selected
    #[clap(short, long, value_hint = clap::ValueHint::FilePath)]
    pub exclude: Option<Vec<String>>,
    /// Run fix command instead of check command
    /// (this is the default behavior unless HK_FIX=0)
    #[clap(short, long, overrides_with = "check")]
    pub fix: bool,
    /// Run on files that match these glob patterns
    #[clap(short, long, value_hint = clap::ValueHint::FilePath)]
    pub glob: Option<Vec<String>>,
    /// Print the plan instead of running the hook
    #[clap(short = 'P', long)]
    pub plan: bool,
    /// Run only specific step(s)
    #[clap(short = 'S', long)]
    pub step: Vec<String>,
    /// Abort on first failure
    #[clap(long, overrides_with = "no_fail_fast")]
    pub fail_fast: bool,
    #[clap(long)]
    pub from_ref: Option<String>,
    /// Start reference for checking files (requires --to-ref)
    /// Continue on failures (opposite of --fail-fast)
    #[clap(long, overrides_with = "fail_fast")]
    pub no_fail_fast: bool,
    /// Disable auto-staging of fixed files
    #[clap(long, overrides_with = "stage")]
    pub no_stage: bool,
    /// Skip specific step(s)
    #[clap(long, value_name = "STEP")]
    pub skip_step: Vec<String>,
    /// Enable auto-staging of fixed files
    #[clap(long, overrides_with = "no_stage")]
    pub stage: bool,
    /// Stash method to use for git hooks
    #[clap(long, value_parser = ["git", "patch-file", "none"])]
    pub stash: Option<String>,
    /// Display statistics about files matching each step
    #[clap(long)]
    pub stats: bool,
    /// End reference for checking files (requires --from-ref)
    #[clap(long)]
    pub to_ref: Option<String>,
    /// Prefilled tera context
    #[clap(skip)]
    pub tctx: Context,
}

impl HookOptions {
    pub fn should_stage(&self) -> Option<bool> {
        if self.stage {
            Some(true)
        } else if self.no_stage {
            Some(false)
        } else {
            None
        }
    }

    pub(crate) async fn run(self, name: &str) -> Result<()> {
        let config = Config::get()?;
        match config.hooks.get(name) {
            Some(hook) => {
                if self.stats {
                    hook.stats(self, name).await?;
                } else if self.plan {
                    hook.plan(self).await?;
                } else {
                    hook.run(self).await?;
                }
                Ok(())
            }
            None => Err(eyre::eyre!("Hook {} not found", name)),
        }
    }
}
