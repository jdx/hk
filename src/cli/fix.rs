use crate::hook_options::HookOptions;

/// Fixes code
#[derive(clap::Args)]
#[clap(visible_alias = "f")]
pub struct Fix {
    #[clap(flatten)]
    pub(crate) hook: HookOptions,
}
