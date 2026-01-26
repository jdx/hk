use crate::hook_options::HookOptions;

/// Checks code
#[derive(clap::Args)]
#[clap(visible_alias = "c")]
pub struct Check {
    #[clap(flatten)]
    pub(crate) hook: HookOptions,
}
