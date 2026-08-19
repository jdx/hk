use crate::hook_options::HookOptions;

/// Checks code
#[derive(usage_rs::Args)]
pub struct Check {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
