use crate::hook_options::HookOptions;

/// Run the check hook to find problems without modifying files
#[derive(usage_rs::Args)]
pub struct Check {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
