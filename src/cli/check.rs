use crate::hook_options::HookOptions;

/// Fixes code
#[derive(clap::Args)]
#[clap(visible_alias = "c")]
pub struct Check {
    #[clap(flatten)]
    pub(crate) hook: HookOptions,
}
