use std::io::IsTerminal;
use std::io::Read;
use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::{Result, git::Git};
use crate::{config::Config, step::CheckType};
use crate::{
    env,
    step::{RunType, Step},
};

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "ph")]
pub struct PrePush {
    /// Remote name
    remote: Option<String>,
    /// Remote URL
    url: Option<String>,
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

#[derive(Debug)]
struct PrePushRefs {
    to: (String, String),
    from: (String, String),
}

impl From<&str> for PrePushRefs {
    fn from(line: &str) -> Self {
        let parts: Vec<&str> = line.split_whitespace().collect();
        PrePushRefs {
            to: (parts[0].to_string(), parts[1].to_string()),
            from: (parts[2].to_string(), parts[3].to_string()),
        }
    }
}

impl PrePush {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        if env::HK_SKIP_HOOK.contains("pre-push") {
            warn!("pre-push: skipping hook due to HK_SKIP_HOOK");
            return Ok(());
        }
        let to_be_updated_refs = if std::io::stdin().is_terminal() {
            vec![]
        } else {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            input
                .lines()
                .filter(|line| !line.is_empty())
                .map(PrePushRefs::from)
                .collect::<Vec<_>>()
        };
        let mut repo = Git::new()?;
        let run_type = RunType::Check(CheckType::Check);

        let from_ref = match &self.to_ref {
            Some(to_ref) => to_ref.clone(),
            None if !to_be_updated_refs.is_empty() => to_be_updated_refs[0].from.1.clone(),
            None => {
                let remote = self.remote.as_deref().unwrap_or("origin");
                let branch = repo.current_branch()?.unwrap_or("HEAD".to_string());
                format!("refs/remotes/{remote}/{branch}")
            }
        };
        let to_ref = self
            .from_ref
            .clone()
            .or(if !to_be_updated_refs.is_empty() {
                Some(to_be_updated_refs[0].to.1.clone())
            } else {
                None
            })
            .unwrap_or("HEAD".to_string());
        debug!("from_ref: {}, to_ref: {}", from_ref, to_ref);

        if !self.all {
            repo.stash_unstaged(self.stash)?;
        }
        static HOOK: LazyLock<IndexMap<String, Step>> = LazyLock::new(Default::default);
        let hook = config.hooks.get("pre-push").unwrap_or(&HOOK);
        let mut result = config
            .run_hook(
                self.all,
                hook,
                run_type,
                &repo,
                &self.linter,
                Some(&from_ref),
                Some(&to_ref),
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
