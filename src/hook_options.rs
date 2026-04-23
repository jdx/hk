use crate::{Result, config::Config, git::Git, settings::Settings, tera::Context};

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
    /// Output the plan as JSON when combined with --plan or --why
    #[clap(short = 'J', long)]
    pub json: bool,
    /// Print the plan instead of running the hook
    #[clap(short = 'P', long)]
    pub plan: bool,
    /// Run only specific step(s)
    #[clap(short = 'S', long)]
    pub step: Vec<String>,
    /// Show detailed reasons for inclusion/exclusion. Pass a step name to focus on one step, or omit the value to show reasons for all steps. Implies --plan.
    #[clap(short = 'W', long, value_name = "STEP", num_args = 0..=1, default_missing_value = "")]
    pub why: Option<String>,
    /// Abort on first failure
    #[clap(long, overrides_with = "no_fail_fast")]
    pub fail_fast: bool,
    /// Invoked by an installed git hook — gracefully exit 0 when no hk.pkl is
    /// present or the event isn't defined. Set automatically by `hk install`.
    #[clap(long, hide = true)]
    pub from_hook: bool,
    /// Start reference for checking files (requires --to-ref)
    #[clap(long)]
    pub from_ref: Option<String>,
    /// Continue on failures (opposite of --fail-fast)
    #[clap(long, overrides_with = "fail_fast")]
    pub no_fail_fast: bool,
    /// Disable auto-staging of fixed files
    #[clap(long, overrides_with = "stage")]
    pub no_stage: bool,
    /// Check only files changed in the current PR/branch (shortcut for --from-ref DEFAULT_BRANCH --to-ref HEAD)
    #[clap(long, conflicts_with_all = &["files", "all", "from_ref", "glob", "to_ref"])]
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
        // Under `--from-hook`, short-circuit *before* loading the config. A
        // broken user-global hkrc (or missing `pkl`) shouldn't fail every
        // `git commit` in a repo that doesn't even use hk — which is the
        // main risk under `hk install --global`.
        if self.from_hook && !Config::project_config_exists() {
            log::debug!("no hk config found for {name}, skipping (--from-hook)");
            return Ok(());
        }
        let config = Config::get()?;
        if self.pr {
            let repo = Git::new()?;
            let default_branch = config
                .default_branch
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| repo.default_branch().unwrap_or_else(|_| "main".to_string()));
            self.from_ref = Some(default_branch);
            self.to_ref = Some("HEAD".to_string());
        }
        // Validate --json. Skip when the user passed --trace (or has
        // HK_TRACE/HK_JSON set) — in that case the global --json flag
        // controls trace output and legitimately populates this field too.
        if self.json
            && !self.plan
            && self.why.is_none()
            && !Settings::cli_trace()
            && !*crate::env::HK_JSON
            && !matches!(*crate::env::HK_TRACE, crate::env::TraceMode::Json)
        {
            return Err(eyre::eyre!("--json requires --plan or --why"));
        }
        match config.hooks.get(name) {
            Some(hook) => {
                if self.stats {
                    hook.stats(self, name).await?;
                } else if self.plan || self.why.is_some() {
                    hook.plan(self).await?;
                } else {
                    hook.run(self).await?;
                }
                Ok(())
            }
            None => {
                if self.from_hook {
                    log::debug!(
                        "hook '{name}' not defined in {}, skipping (--from-hook)",
                        config.path.display()
                    );
                    return Ok(());
                }
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
