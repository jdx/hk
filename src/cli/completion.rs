use crate::Result;
use duct::cmd;

/// Generates shell completion scripts
#[derive(Debug, clap::Args)]
#[clap()]
pub struct Completion {
    /// The shell to generate completion for
    #[clap()]
    shell: String,
}

impl Completion {
    pub async fn run(&self) -> Result<()> {
        cmd!(
            "usage",
            "g",
            "completion",
            &self.shell,
            "hk",
            "--usage-cmd",
            "hk usage",
        )
        .run()?;
        Ok(())
    }
}
