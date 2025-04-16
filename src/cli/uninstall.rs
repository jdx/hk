use std::path::PathBuf;

use crate::Result;

/// Removes hk hooks from the current git repository
#[derive(Debug, clap::Args)]
pub struct Uninstall {}

impl Uninstall {
    pub async fn run(&self) -> Result<()> {
        let hooks = PathBuf::from(".git/hooks");
        for p in xx::file::ls(&hooks)? {
            let content = match xx::file::read_to_string(&p) {
                Ok(content) => content,
                Err(e) => {
                    debug!("failed to read hook: {e}");
                    continue;
                }
            };
            let is_hk_hook = content.trim_end().lines().last().is_some_and(|line| {
                line.starts_with("exec hk run") || line.starts_with("exec mise x -- hk run")
            });
            if is_hk_hook {
                xx::file::remove_file(&p)?;
                info!("removed hook: {}", xx::file::display_path(&p));
            }
        }
        Ok(())
    }
}
