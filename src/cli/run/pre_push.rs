use std::io::IsTerminal;
use std::io::Read;

use crate::hook_options::HookOptions;
use crate::{Result, git::Git};

#[derive(clap::Args)]
#[clap(visible_alias = "pp")]
pub struct PrePush {
    /// Remote name
    remote: Option<String>,
    /// Remote URL
    url: Option<String>,
    #[clap(flatten)]
    hook: HookOptions,
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
    pub async fn run(mut self) -> Result<()> {
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

        self.hook.from_ref = Some(match &self.hook.from_ref {
            Some(to_ref) => to_ref.clone(),
            None if !to_be_updated_refs.is_empty() => to_be_updated_refs[0].from.1.clone(),
            None => {
                let remote = self.remote.as_deref().unwrap_or("origin");
                let repo = Git::new()?; // TODO: remove this extra repo creation
                repo.matching_remote_branch(remote)?
                    .unwrap_or(format!("refs/remotes/{remote}/HEAD"))
            }
        });
        self.hook.to_ref = Some(
            self.hook
                .to_ref
                .clone()
                .or(if !to_be_updated_refs.is_empty() {
                    Some(to_be_updated_refs[0].to.1.clone())
                } else {
                    None
                })
                .unwrap_or("HEAD".to_string()),
        );
        debug!(
            "from_ref: {}, to_ref: {}",
            self.hook.from_ref.as_ref().unwrap(),
            self.hook.to_ref.as_ref().unwrap()
        );

        self.hook.run("pre-push").await
    }
}
