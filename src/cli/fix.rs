use crate::hook_options::HookOptions;

/// Fixes code
#[derive(usage_derive::Args)]
pub struct Fix {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
