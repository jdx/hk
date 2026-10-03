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
    /// Windows Command Prompt
    Cmd,
    /// Windows PowerShell
    PowerShell,
    /// Other/unknown shell
    #[allow(unused)]
    Other(String),
}

/// Environment variable holding a literal `%` for cmd.exe quoting.
pub const CMD_PERCENT_VAR: &str = "HK_CMD_PERCENT";

/// Reserves `HK_CMD_PERCENT` for hk when `enabled`: drops any user definition
/// (names are case-insensitive on Windows) and appends hk's value last so it
/// always wins. `ShellType::Cmd::quote` relies on it expanding to a literal `%`.
pub fn with_cmd_percent(mut env: Vec<(String, String)>, enabled: bool) -> Vec<(String, String)> {
    if enabled {
        env.retain(|(key, _)| !key.eq_ignore_ascii_case(CMD_PERCENT_VAR));
        env.push((CMD_PERCENT_VAR.to_string(), "%".to_string()));
    }
    env
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
        self.quote_impl(s, false)
    }

    /// Like [`quote`](Self::quote), but for text shown to users. cmd.exe gets a
    /// plain `%` instead of the internal `%HK_CMD_PERCENT%` placeholder, since
    /// the variable is only defined in the child hk spawns, not in the shell
    /// where a suggested command might be copied.
    pub fn quote_display(&self, s: &str) -> String {
        self.quote_impl(s, true)
    }

    fn quote_impl(&self, s: &str, display: bool) -> String {
        match self {
            ShellType::Bash | ShellType::Zsh => s.quoted(shell_quote::Bash),
            ShellType::Fish => s.quoted(shell_quote::Fish),
            ShellType::Cmd => {
                // Windows cmd.exe quoting: wrap in double quotes, escape special characters
                // - Double quotes are escaped as ""
                // - Percent signs become `%HK_CMD_PERCENT%`. `%%` only collapses inside
                //   batch files; on a `cmd /c` command line it stays `%%`, so a file
                //   named `100%.txt` arrived as `100%%.txt`. cmd expands variables in a
                //   single pass, so the injected variable yields a literal `%` that is
                //   never re-expanded. The runner defines it for cmd.exe steps.
                let percent = if display {
                    "%".to_string()
                } else {
                    format!("%{CMD_PERCENT_VAR}%")
                };
                let escaped = s.replace('%', &percent).replace('"', "\"\"");
                format!("\"{}\"", escaped)
            }
            ShellType::PowerShell => {
                // PowerShell quoting: wrap in single quotes, double internal single quotes
                let escaped = s.replace('\'', "''");
                format!("'{}'", escaped)
            }
            ShellType::Dash | ShellType::Sh | ShellType::Other(_) => {
                let mut o = vec![];
                shell_quote::Sh::quote_into(s, &mut o);
                String::from_utf8(o).unwrap_or_default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_quote_expands_percent_through_variable() {
        assert_eq!(
            ShellType::Cmd.quote("100%.txt"),
            "\"100%HK_CMD_PERCENT%.txt\""
        );
        assert_eq!(
            ShellType::Cmd.quote("a%PATH%\"b"),
            "\"a%HK_CMD_PERCENT%PATH%HK_CMD_PERCENT%\"\"b\""
        );
    }

    #[test]
    fn cmd_percent_overrides_user_definitions() {
        let env = vec![
            ("A".to_string(), "1".to_string()),
            ("hk_cmd_percent".to_string(), "evil".to_string()),
            (CMD_PERCENT_VAR.to_string(), "x".to_string()),
        ];
        let out = with_cmd_percent(env.clone(), true);
        assert_eq!(
            out,
            vec![
                ("A".to_string(), "1".to_string()),
                (CMD_PERCENT_VAR.to_string(), "%".to_string()),
            ]
        );
        assert_eq!(with_cmd_percent(env.clone(), false), env);
    }

    #[test]
    fn quote_display_hides_placeholder() {
        assert_eq!(ShellType::Cmd.quote_display("100%.txt"), "\"100%.txt\"");
        assert_eq!(
            ShellType::Bash.quote_display("a b"),
            ShellType::Bash.quote("a b")
        );
    }
}
