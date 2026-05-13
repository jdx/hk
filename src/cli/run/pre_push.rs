use std::io::IsTerminal;
use std::io::Read;

use crate::hook_options::HookOptions;
use crate::{
    Result,
    git::{Git, is_zero_sha},
};

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
        self.hook.tctx.insert(
            "hook_args",
            &format!(
                "{} {}",
                self.remote.as_deref().unwrap_or(""),
                self.url.as_deref().unwrap_or("")
            ),
        );
        let to_be_updated_refs = if std::io::stdin().is_terminal() {
            self.hook.tctx.insert("hook_stdin", "");
            vec![]
        } else {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            self.hook.tctx.insert("hook_stdin", &input);
            // Note: we deliberately keep deletions (local sha all-zeros) in
            // the list. The downstream EMPTY_REF guard in hook.rs detects
            // `to_ref` of all-zeros and short-circuits to an empty file set
            // — dropping deletions here would route them through
            // `files_between_refs(default_branch, "HEAD")` and lint
            // unrelated files.
            input
                .lines()
                .filter(|line| !line.is_empty())
                .map(PrePushRefs::from)
                .collect::<Vec<_>>()
        };
        trace!("to_be_updated_refs: {to_be_updated_refs:?}");

        self.hook.from_ref = Some(match &self.hook.from_ref {
            Some(to_ref) => to_ref.clone(),
            None if !to_be_updated_refs.is_empty()
                && !is_zero_sha(&to_be_updated_refs[0].from.1) =>
            {
                to_be_updated_refs[0].from.1.clone()
            }
            None => {
                // Either no refs were provided on stdin, or the first ref is
                // a new-branch push (remote sha is all-zeros). Fall back to
                // the remote-tracking branch if it exists, then to the
                // repository's default branch on the target remote.
                let remote = self.remote.as_deref().unwrap_or("origin");
                let repo = Git::new()?; // TODO: remove this extra repo creation
                if let Some(rb) = repo.matching_remote_branch(remote)? {
                    rb
                } else if remote == "origin" {
                    repo.resolve_default_branch()
                } else {
                    // resolve_default_branch is internally hardcoded to
                    // origin, so for a non-origin remote derive the bare
                    // branch name and rebind it onto the actual remote.
                    let default = repo.resolve_default_branch();
                    let bare = default
                        .rsplit_once('/')
                        .map(|(_, name)| name)
                        .unwrap_or(&default);
                    format!("refs/remotes/{remote}/{bare}")
                }
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
