use crate::{Result, git_util};

/// Removes hk hooks from the current git repository
#[derive(Debug, clap::Args)]
pub struct Uninstall {}

impl Uninstall {
    pub async fn run(&self) -> Result<()> {
        let git_path = git_util::find_git_path()?;
        let hooks = git_util::resolve_git_hooks_dir(&git_path)?;
        for p in xx::file::ls(&hooks)? {
            let content = match xx::file::read_to_string(&p) {
                Ok(content) => content,
                Err(e) => {
                    debug!("failed to read hook: {e}");
                    continue;
                }
            };
            let is_hk_hook = content.contains("hk run");
            if is_hk_hook {
                xx::file::remove_file(&p)?;
                info!("removed hook: {}", xx::file::display_path(&p));
            }
        }
        Ok(())
    }
}
