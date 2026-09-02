use crate::hook_options::HookOptions;

/// Run the fix hook
///
/// Runs each step's fix command to modify files in place. Passing `--check`,
/// or setting `HK_CHECK=1` or `HK_FIX=0`, runs the read-only check commands
/// instead.
#[derive(usage_rs::Args)]
pub struct Fix {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
