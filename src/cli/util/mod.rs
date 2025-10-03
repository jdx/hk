mod mixed_line_ending;
mod trailing_whitespace;

pub use mixed_line_ending::MixedLineEnding;
pub use trailing_whitespace::TrailingWhitespace;

use crate::Result;

/// Utility commands for file operations
#[derive(Debug, clap::Args)]
pub struct Util {
    #[clap(subcommand)]
    command: UtilCommands,
}

#[derive(Debug, clap::Subcommand)]
enum UtilCommands {
    /// Detect and fix mixed line endings
    MixedLineEnding(MixedLineEnding),
    /// Check for and optionally fix trailing whitespace
    TrailingWhitespace(TrailingWhitespace),
}

impl Util {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            UtilCommands::MixedLineEnding(cmd) => cmd.run().await,
            UtilCommands::TrailingWhitespace(cmd) => cmd.run().await,
        }
    }
}
