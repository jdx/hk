use std::sync::LazyLock;

use indexmap::IndexMap;

use crate::{
    Result,
    config::Config,
    env,
    git::Git,
    step::{CheckType, RunType, Step},
};

mod commit_msg;
mod pre_commit;
mod pre_push;
mod prepare_commit_msg;

/// Run a hook
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "r", verbatim_doc_comment)]
pub struct Run {
    #[clap(subcommand)]
    command: Option<Commands>,
    #[clap(hide = true)]
    other: Option<String>,
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    CommitMsg(commit_msg::CommitMsg),
    PreCommit(pre_commit::PreCommit),
    PrePush(pre_push::PrePush),
    PrepareCommitMsg(prepare_commit_msg::PrepareCommitMsg),
}

impl Run {
    pub async fn run(self) -> Result<()> {
        if let Some(hook) = &self.other {
            return self.other(hook).await;
        }
        match self.command.unwrap() {
            Commands::CommitMsg(cmd) => cmd.run().await,
            Commands::PreCommit(cmd) => cmd.run().await,
            Commands::PrePush(cmd) => cmd.run().await,
            Commands::PrepareCommitMsg(cmd) => cmd.run().await,
        }
    }

    async fn other(&self, hook: &str) -> Result<()> {
        let config = Config::get()?;
        if env::HK_SKIP_HOOK.contains(hook) {
            warn!("{hook}: skipping hook due to HK_SKIP_HOOK");
            return Ok(());
        }
        let mut repo = Git::new()?;
        let run_type = RunType::Check(CheckType::Check);

        static HOOK: LazyLock<IndexMap<String, Step>> = LazyLock::new(Default::default);
        let hook = config.hooks.get(hook).unwrap_or(&HOOK);
        let mut result = config
            .run_hook(
                true,
                hook,
                run_type,
                &repo,
                &[],
                Default::default(),
                None,
                None,
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
