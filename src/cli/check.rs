use crate::hook_options::HookOptions;

/// Run the check hook
///
/// Reports problems without modifying files. If the `check` hook sets
/// `fix = true` in `hk.pkl`, or `--fix` is passed, hk runs fix commands
/// instead and can modify and stage files.
#[derive(usage_rs::Args)]
pub struct Check {
    #[usage(flatten)]
    pub(crate) hook: HookOptions,
}
