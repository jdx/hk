use crate::{Result, config::Config, git::Git, tera::Context};

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
    /// Start reference for checking files (requires --to-ref)
    #[clap(long)]
    pub from_ref: Option<String>,
    /// Continue on failures (opposite of --fail-fast)
    #[clap(long, overrides_with = "fail_fast")]
    pub no_fail_fast: bool,
    /// Disable auto-staging of fixed files
    #[clap(long, overrides_with = "stage")]
    pub no_stage: bool,
    /// Check only files changed in the current PR/branch (shortcut for --from-ref <default-branch> --to-ref HEAD)
    #[clap(long, conflicts_with_all = &["files", "all", "from_ref", "to_ref"])]
    pub pr: bool,
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

    pub(crate) async fn run(mut self, name: &str) -> Result<()> {
        if self.pr {
            let repo = Git::new()?;
            self.from_ref = Some(repo.resolve_default_branch());
            self.to_ref = Some("HEAD".to_string());
        }
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
            None => {
                let hook_names: Vec<&str> = config.hooks.keys().map(|s| s.as_str()).collect();
                let msg = if let Some(suggestion) = xx::suggest::did_you_mean(name, &hook_names) {
                    format!("Hook '{}' not found. {}", name, suggestion)
                } else {
                    format!("Hook '{}' not found", name)
                };
                Err(eyre::eyre!("{}", msg))
            }
        }
    }
}
