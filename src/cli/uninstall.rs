use crate::{Result, cli::install};

/// Removes hk hooks from the current git repository
#[derive(Debug, clap::Args)]
pub struct Uninstall {
    /// Remove hk hooks from the user's global git config (`~/.gitconfig`).
    #[clap(long, verbatim_doc_comment)]
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
