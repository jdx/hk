use crate::hook_options::HookOptions;

/// Checks code
#[derive(usage_derive::Args)]
#[usage(arg, alias = "c")]
pub struct Check {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
