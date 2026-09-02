use crate::hook_options::HookOptions;

/// Run the fix hook
///
/// Runs each step's fix command to modify files in place. Passing `--check`, or
/// setting `HK_FIX=0`, runs each step's check command instead.
#[derive(usage_rs::Args)]
pub struct Fix {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
