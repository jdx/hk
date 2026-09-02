use crate::hook_options::HookOptions;

/// Run the fix hook to fix problems in place
#[derive(usage_rs::Args)]
pub struct Fix {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
