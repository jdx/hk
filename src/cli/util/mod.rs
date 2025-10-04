mod check_added_large_files;
mod check_byte_order_marker;
mod check_case_conflict;
mod check_executables_have_shebangs;
mod check_merge_conflict;
mod check_symlinks;
mod detect_private_key;
mod fix_byte_order_marker;
mod mixed_line_ending;
mod no_commit_to_branch;
mod python_check_ast;
mod python_debug_statements;
mod trailing_whitespace;

pub use check_added_large_files::CheckAddedLargeFiles;
pub use check_byte_order_marker::CheckByteOrderMarker;
pub use check_case_conflict::CheckCaseConflict;
pub use check_executables_have_shebangs::CheckExecutablesHaveShebangs;
pub use check_merge_conflict::CheckMergeConflict;
pub use check_symlinks::CheckSymlinks;
pub use detect_private_key::DetectPrivateKey;
pub use fix_byte_order_marker::FixByteOrderMarker;
pub use mixed_line_ending::MixedLineEnding;
pub use no_commit_to_branch::NoCommitToBranch;
pub use python_check_ast::PythonCheckAst;
pub use python_debug_statements::PythonDebugStatements;
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
    /// Check for large files being added to repository
    CheckAddedLargeFiles(CheckAddedLargeFiles),
    /// Check for UTF-8 byte order marker (BOM)
    CheckByteOrderMarker(CheckByteOrderMarker),
    /// Check for case-insensitive filename conflicts
    CheckCaseConflict(CheckCaseConflict),
    /// Check that executable files have shebangs
    CheckExecutablesHaveShebangs(CheckExecutablesHaveShebangs),
    /// Check for merge conflict markers
    CheckMergeConflict(CheckMergeConflict),
    /// Check for broken symlinks
    CheckSymlinks(CheckSymlinks),
    /// Detect private keys in files
    DetectPrivateKey(DetectPrivateKey),
    /// Remove UTF-8 byte order marker (BOM)
    FixByteOrderMarker(FixByteOrderMarker),
    /// Detect and fix mixed line endings
    MixedLineEnding(MixedLineEnding),
    /// Prevent commits to specific branches
    NoCommitToBranch(NoCommitToBranch),
    /// Check Python files for valid syntax
    PythonCheckAst(PythonCheckAst),
    /// Detect Python debug statements
    PythonDebugStatements(PythonDebugStatements),
    /// Check for and optionally fix trailing whitespace
    TrailingWhitespace(TrailingWhitespace),
}

impl Util {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            UtilCommands::CheckAddedLargeFiles(cmd) => cmd.run().await,
            UtilCommands::CheckByteOrderMarker(cmd) => cmd.run().await,
            UtilCommands::CheckCaseConflict(cmd) => cmd.run().await,
            UtilCommands::CheckExecutablesHaveShebangs(cmd) => cmd.run().await,
            UtilCommands::CheckMergeConflict(cmd) => cmd.run().await,
            UtilCommands::CheckSymlinks(cmd) => cmd.run().await,
            UtilCommands::DetectPrivateKey(cmd) => cmd.run().await,
            UtilCommands::FixByteOrderMarker(cmd) => cmd.run().await,
            UtilCommands::MixedLineEnding(cmd) => cmd.run().await,
            UtilCommands::NoCommitToBranch(cmd) => cmd.run().await,
            UtilCommands::PythonCheckAst(cmd) => cmd.run().await,
            UtilCommands::PythonDebugStatements(cmd) => cmd.run().await,
            UtilCommands::TrailingWhitespace(cmd) => cmd.run().await,
        }
    }
}
