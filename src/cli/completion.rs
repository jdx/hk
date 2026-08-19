use crate::{Result, cli::Cli};

/// Generates shell completion scripts
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Completion {
    /// The shell to generate completion for
    #[usage(arg)]
    shell: String,
}

impl Completion {
    pub async fn run(&self) -> Result<()> {
        let shell = usage_rs::complete::Shell::from_name(&self.shell)
            .ok_or_else(|| eyre::eyre!("unsupported shell: {}", self.shell))?;
        print!("{}", Cli::completion_script(shell));
        Ok(())
    }
}
