mod check_executables_have_shebangs;
mod check_merge_conflict;
mod check_symlinks;
mod mixed_line_ending;
mod trailing_whitespace;

pub use check_executables_have_shebangs::CheckExecutablesHaveShebangs;
pub use check_merge_conflict::CheckMergeConflict;
pub use check_symlinks::CheckSymlinks;
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
    /// Check that executable files have shebangs
    CheckExecutablesHaveShebangs(CheckExecutablesHaveShebangs),
    /// Check for merge conflict markers
    CheckMergeConflict(CheckMergeConflict),
    /// Check for broken symlinks
    CheckSymlinks(CheckSymlinks),
    /// Detect and fix mixed line endings
    MixedLineEnding(MixedLineEnding),
    /// Check for and optionally fix trailing whitespace
    TrailingWhitespace(TrailingWhitespace),
}

impl Util {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            UtilCommands::CheckExecutablesHaveShebangs(cmd) => cmd.run().await,
            UtilCommands::CheckMergeConflict(cmd) => cmd.run().await,
            UtilCommands::CheckSymlinks(cmd) => cmd.run().await,
            UtilCommands::MixedLineEnding(cmd) => cmd.run().await,
            UtilCommands::TrailingWhitespace(cmd) => cmd.run().await,
        }
    }
}
