use crate::config::Config;
use crate::{env, step::RunType};
use crate::{git::Git, Result};

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "ph")]
pub struct PrePush {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Run fix command instead of run command
    /// This is the default behavior unless HK_FIX=0
    #[clap(short, long, overrides_with = "check")]
    fix: bool,
    /// Run check command instead of fix command
    #[clap(short, long, overrides_with = "fix")]
    check: bool,
    /// Remote name
    remote: String,
    /// Force stashing even if it's disabled via HK_STASH
    #[clap(long)]
    stash: bool,
    /// Remote URL
    url: String,
}

#[allow(unreachable_code)]
impl PrePush {
    pub async fn run(&self) -> Result<()> {
        unimplemented!(
            "pre-push is not yet implemented. We need support for --from-ref and --to-ref"
        );
        let config = Config::get()?;
        if env::HK_SKIP_HOOK.contains("pre-push") {
            warn!("pre-push: skipping hook due to HK_SKIP_HOOK");
            return Ok(());
        }
        let mut repo = Git::new()?;
        let run_type = if self.all {
            if !self.check && (self.fix || *env::HK_FIX) {
                RunType::FixAll
            } else {
                RunType::CheckAll
            }
        } else if !self.check && (self.fix || *env::HK_FIX) {
            RunType::Fix
        } else {
            RunType::Check
        };
        if !self.all {
            repo.stash_unstaged(self.stash)?;
        }
        let mut result = if let Some(hook) = &config.pre_push {
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
