use crate::{env, Result};

#[derive(Debug, clap::Args)]
pub struct Clear {}

impl Clear {
    pub async fn run(&self) -> Result<()> {
        if env::HK_CACHE_DIR.exists() {
            xx::file::remove_dir_all(&*env::HK_CACHE_DIR)?;
            xx::file::mkdirp(&*env::HK_CACHE_DIR)?;
        }
        Ok(())
    }
}
