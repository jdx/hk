use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::{Result, git::Git};
use crate::{
    config::Config,
    env,
    step::{CheckType, RunType, Step},
};

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

impl PreCommit {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        if env::HK_SKIP_HOOK.contains("pre-commit") {
            warn!("pre-commit: skipping hook due to HK_SKIP_HOOK");
            return Ok(());
        }
        let mut repo = Git::new()?;
        let run_type = if !self.check && (self.fix || *env::HK_FIX) {
            RunType::Fix
        } else {
            RunType::Check(CheckType::Check)
        };

        // Check if both from_ref and to_ref are provided or neither
        if (self.from_ref.is_some() && self.to_ref.is_none())
            || (self.from_ref.is_none() && self.to_ref.is_some())
        {
            return Err(eyre::eyre!(
                "Both --from-ref and --to-ref must be provided together"
            ));
        }

        if !self.all {
            repo.stash_unstaged(self.stash)?;
        }
        static HOOK: LazyLock<IndexMap<String, Step>> = LazyLock::new(Default::default);
        let hook = config.hooks.get("pre-commit").unwrap_or(&HOOK);
        let mut result = config
            .run_hook(
                self.all,
                hook,
                run_type,
                &repo,
                &self.linter,
                Default::default(),
                self.from_ref.as_deref(),
                self.to_ref.as_deref(),
            )
            .await;

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
