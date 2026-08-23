use crate::hook_options::HookOptions;

/// Fixes code
#[derive(usage_rs::Args)]
pub struct Fix {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
