use crate::{Result, cli::install};

/// Remove hk hooks
///
/// Removes hk's hooks from the current git repository, clearing both legacy
/// `.git/hooks/` shims and config-based `hook.*` entries. Pass `--global` to
/// remove them from the user's `~/.gitconfig` instead.
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "destructive")]
pub struct Uninstall {
    /// Remove hk hooks from the user's global git config (`~/.gitconfig`).
    #[usage(long, verbatim_doc_comment)]
    global: bool,
}

impl Uninstall {
    pub async fn run(&self) -> Result<()> {
        if self.global {
            install::remove_config_entries("--global")?;
            info!("removed hk hooks from ~/.gitconfig");
            return Ok(());
        }
        // Clean both legacy script shims and config-based entries so the
        // uninstall is complete regardless of which mode the user had.
        install::remove_local_shims()?;
        install::remove_config_entries("--local")?;
        info!("removed hk hooks from this repository");
        Ok(())
    }
}
