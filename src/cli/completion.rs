use crate::Result;

/// Generates shell completion scripts
#[derive(Debug, usage_derive::Args)]
#[usage(arg)]
pub struct Completion {
    /// The shell to generate completion for
    #[usage(arg)]
    shell: String,
}

impl Completion {
    pub async fn run(&self) -> Result<()> {
        xx::process::cmd(
            "usage",
            [
                "g",
                "completion",
                &self.shell,
                "hk",
                "--usage-cmd",
                "hk usage",
            ],
        )
        .run()?;
        Ok(())
    }
}
