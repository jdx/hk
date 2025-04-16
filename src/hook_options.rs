use crate::{Result, config::Config, tera::Context};

#[derive(clap::Args)]
pub(crate) struct HookOptions {
    /// Run on specific files
    #[clap(conflicts_with_all = &["all", "fix", "check"])]
    pub files: Option<Vec<String>>,
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    pub all: bool,
    /// Run fix command instead of run command
    /// This is the default behavior unless HK_FIX=0
    #[clap(short, long, overrides_with = "check")]
    pub fix: bool,
    /// Run run command instead of fix command
    #[clap(short, long, overrides_with = "fix")]
    pub check: bool,
    /// Exclude files that otherwise would have been selected
    #[clap(short, long)]
    pub exclude: Option<Vec<String>>,
    /// Exclude files that match these glob patterns that otherwise would have been selected
    #[clap(long)]
    pub exclude_glob: Option<Vec<String>>,
    /// Start reference for checking files (requires --to-ref)
    #[clap(long)]
    pub from_ref: Option<String>,
    /// End reference for checking files (requires --from-ref)
    #[clap(long)]
    pub to_ref: Option<String>,
    /// Run on files that match these glob patterns
    #[clap(short, long)]
    pub glob: Option<Vec<String>>,
    /// Print the plan instead of running the hook
    #[clap(short = 'P', long)]
    pub plan: bool,
    /// Run specific step(s)
    #[clap(short = 'S', long)]
    pub step: Vec<String>,
    /// Prefilled tera context
    #[clap(skip)]
    pub tctx: Context,
}

impl HookOptions {
    pub(crate) async fn run(self, name: &str) -> Result<()> {
        let config = Config::get()?;
        match config.hooks.get(name) {
            Some(hook) => hook.run(self).await,
            None => Err(eyre::eyre!("Hook {} not found", name)),
        }
    }
}
