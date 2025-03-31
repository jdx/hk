use std::io::IsTerminal;
use std::io::Read;

use crate::config::Config;
use crate::{Result, git::Git};

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
        let to_be_updated_refs = if std::io::stdin().is_terminal() {
            vec![]
        } else {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            input
                .lines()
                .filter(|line| !line.is_empty())
                .map(PrePushRefs::from)
                .filter(|refs| {
                    // git uses this if the remote ref does not exist, we can just ignore it in that case and default to origin/HEAD
                    refs.to.1 == "0000000000000000000000000000000000000000"
                        && refs.from.1 != "0000000000000000000000000000000000000000"
                })
                .collect::<Vec<_>>()
        };
        trace!("to_be_updated_refs: {:?}", to_be_updated_refs);

        let from_ref = match &self.from_ref {
            Some(to_ref) => to_ref.clone(),
            None if !to_be_updated_refs.is_empty() => to_be_updated_refs[0].from.1.clone(),
            None => {
                let remote = self.remote.as_deref().unwrap_or("origin");
                let repo = Git::new()?; // TODO: remove this extra repo creation
                repo.matching_remote_branch(remote)?
                    .unwrap_or(format!("refs/remotes/{remote}/HEAD"))
            }
        };
        let to_ref = self
            .to_ref
            .clone()
            .or(if !to_be_updated_refs.is_empty() {
                Some(to_be_updated_refs[0].to.1.clone())
            } else {
                None
            })
            .unwrap_or("HEAD".to_string());
        debug!("from_ref: {}, to_ref: {}", from_ref, to_ref);

        config
            .run_hook(
                self.all,
                "pre-push",
                &self.linter,
                Default::default(),
                Some(&from_ref),
                Some(&to_ref),
            )
            .await
    }
}
