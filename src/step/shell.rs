//! Shell type detection and quoting utilities.
//!
//! This module provides shell-specific functionality for:
//! - Detecting which shell is being used for a step
//! - Properly quoting strings for different shell types

use shell_quote::{QuoteInto, QuoteRefExt};

/// The type of shell used to execute step commands.
///
/// Different shells have different quoting rules, so knowing the shell type
/// allows for proper escaping of file paths and arguments.
pub enum ShellType {
    /// GNU Bash
    Bash,
    /// Dash (Debian Almquist Shell)
    Dash,
    /// Fish shell
    Fish,
    /// POSIX sh
    Sh,
    /// Z shell
    Zsh,
    /// Other/unknown shell
    #[allow(unused)]
    Other(String),
}

impl ShellType {
    /// Quote a string appropriately for this shell type.
    ///
    /// This ensures special characters are properly escaped so the string
    /// can be safely passed as an argument to shell commands.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to quote
    ///
    /// # Returns
    ///
    /// A properly quoted string for the target shell
    pub fn quote(&self, s: &str) -> String {
        match self {
            ShellType::Bash | ShellType::Zsh => s.quoted(shell_quote::Bash),
            ShellType::Fish => s.quoted(shell_quote::Fish),
            ShellType::Dash | ShellType::Sh | ShellType::Other(_) => {
                let mut o = vec![];
                shell_quote::Sh::quote_into(s, &mut o);
                String::from_utf8(o).unwrap_or_default()
            }
        }
    }
}
