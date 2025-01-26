use crate::{env, Result};

#[derive(Debug, clap::Args)]
pub struct Clear {}

impl Clear {
    pub async fn run(&self) -> Result<()> {
        if env::ANGLER_CACHE_DIR.exists() {
            xx::file::remove_dir_all(&*env::ANGLER_CACHE_DIR)?;
            xx::file::mkdirp(&*env::ANGLER_CACHE_DIR)?;
        }
        Ok(())
    }
}
