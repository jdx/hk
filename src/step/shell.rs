//! Shell type detection and quoting utilities.
//!
//! This module provides shell-specific functionality for:
//! - Detecting which shell is being used for a step
//! - Properly quoting strings for different shell types

use shell_quote::{QuoteInto, QuoteRefExt};

/// Split a step's `shell` setting into the program and its arguments.
///
/// Words are separated by whitespace, and quotes group words, so a program path
/// with a space can be quoted. Elsewhere the value is split the way a POSIX
/// shell would. On Windows it follows the rules of `CommandLineToArgvW`:
/// backslashes are ordinary characters, so `C:\tools\bash.exe` keeps its path,
/// except before a double quote, where `2n` backslashes give `n` and the quote
/// groups, and `2n+1` give `n` and a literal quote. That lets
/// `sh -c "printf \"hello world\""` pass `printf "hello world"` as one argument.
/// Single quotes also group, with no escapes inside. If that reading leaves a
/// quote open, as in `pwsh.exe -WorkingDirectory "C:\My Projects\" -Command`,
/// backslashes are read as fully literal instead, and only then does the value
/// fall back to whitespace splitting.
pub(crate) fn split_shell(shell: &str) -> Vec<String> {
    split_shell_for(shell, cfg!(windows))
}

fn split_shell_for(shell: &str, windows: bool) -> Vec<String> {
    let whitespace = || shell.split_whitespace().map(str::to_string).collect();
    if !shell.contains(['"', '\'']) {
        return whitespace();
    }
    let words = if windows {
        split_windows(shell, true).or_else(|| split_windows(shell, false))
    } else {
        shell_words::split(shell).ok()
    };
    words.unwrap_or_else(whitespace)
}

/// Split like `CommandLineToArgvW`, or with backslashes fully literal when
/// `backslash_escapes` is off. `None` when a quote is left open.
fn split_windows(shell: &str, backslash_escapes: bool) -> Option<Vec<String>> {
    let chars: Vec<char> = shell.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut double = false;
    let mut single = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if single {
            if c == '\'' {
                single = false;
            } else {
                word.push(c);
            }
        } else if c == '\\' && backslash_escapes {
            let start = i;
            while i < chars.len() && chars[i] == '\\' {
                i += 1;
            }
            let run = i - start;
            in_word = true;
            if chars.get(i) == Some(&'"') {
                word.extend(std::iter::repeat_n('\\', run / 2));
                if run % 2 == 1 {
                    word.push('"');
                } else {
                    double = !double;
                }
                i += 1;
            } else {
                word.extend(std::iter::repeat_n('\\', run));
            }
            continue;
        } else if c == '"' {
            double = !double;
            in_word = true;
        } else if c == '\'' && !double {
            single = true;
            in_word = true;
        } else if c.is_whitespace() && !double {
            if in_word {
                words.push(std::mem::take(&mut word));
                in_word = false;
            }
        } else {
            word.push(c);
            in_word = true;
        }
        i += 1;
    }
    if double || single {
        return None;
    }
    if in_word {
        words.push(word);
    }
    Some(words)
}

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

    fn windows(shell: &str) -> Vec<String> {
        split_shell_for(shell, true)
    }

    fn posix(shell: &str) -> Vec<String> {
        split_shell_for(shell, false)
    }

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

    #[test]
    fn splits_on_whitespace() {
        assert_eq!(posix("bash -o errexit -c"), ["bash", "-o", "errexit", "-c"]);
        assert!(posix("   ").is_empty());
    }

    #[test]
    fn keeps_the_backslashes_of_an_unquoted_windows_path() {
        assert_eq!(posix(r"C:\tools\sh.exe -c"), [r"C:\tools\sh.exe", "-c"]);
    }

    #[test]
    fn honors_quotes_around_a_path_with_spaces() {
        assert_eq!(
            posix(r#""C:\Program Files\Git\usr\bin\sh.exe" -o errexit -c"#),
            [
                r"C:\Program Files\Git\usr\bin\sh.exe",
                "-o",
                "errexit",
                "-c"
            ]
        );
        assert_eq!(
            posix(r"'C:\Program Files\Git\bin\sh.exe' -c"),
            [r"C:\Program Files\Git\bin\sh.exe", "-c"]
        );
    }

    #[test]
    fn falls_back_to_whitespace_when_a_quote_is_unbalanced() {
        assert_eq!(posix(r#"sh "-c"#), ["sh", r#""-c"#]);
    }

    #[test]
    fn windows_keeps_backslashes_when_an_argument_is_quoted() {
        assert_eq!(
            windows(r#"C:\tools\bash.exe --rcfile "C:/My Scripts/bashrc" -c"#),
            [
                r"C:\tools\bash.exe",
                "--rcfile",
                "C:/My Scripts/bashrc",
                "-c"
            ]
        );
    }

    #[test]
    fn windows_passes_an_escaped_quote_script_as_one_argument() {
        assert_eq!(
            windows(r##"sh -c "printf \"hello world\"""##),
            ["sh", "-c", r##"printf "hello world""##]
        );
        // 2n+1 backslashes: n backslashes and a literal quote; 2n: n and the quote groups.
        assert_eq!(windows(r#"a \\\"b"#), ["a", r#"\"b"#]);
        assert_eq!(windows(r#"a "b\\" c"#), ["a", r"b\", "c"]);
    }

    #[test]
    fn windows_groups_a_quoted_path_with_spaces() {
        assert_eq!(
            windows(r#""C:\Program Files\Git\usr\bin\sh.exe" -o errexit -c"#),
            [
                r"C:\Program Files\Git\usr\bin\sh.exe",
                "-o",
                "errexit",
                "-c"
            ]
        );
        assert_eq!(
            windows(r"'C:\Program Files\Git\bin\sh.exe' -c"),
            [r"C:\Program Files\Git\bin\sh.exe", "-c"]
        );
    }

    #[test]
    fn windows_ends_a_quoted_path_at_a_trailing_backslash_pair() {
        assert_eq!(windows(r#""C:\dir\\" -c"#), [r"C:\dir\", "-c"]);
    }

    #[test]
    fn windows_keeps_a_posix_style_value() {
        assert_eq!(
            windows("bash -o errexit -c"),
            ["bash", "-o", "errexit", "-c"]
        );
        assert_eq!(windows(r#"sh -c "set -e""#), ["sh", "-c", "set -e"]);
        assert_eq!(windows(r#"sh "-c"#), ["sh", r#""-c"#]);
    }

    #[test]
    fn windows_keeps_a_quoted_directory_with_a_trailing_backslash() {
        assert_eq!(
            windows(r#"pwsh.exe -WorkingDirectory "C:\My Projects\" -Command"#),
            [
                "pwsh.exe",
                "-WorkingDirectory",
                r"C:\My Projects\",
                "-Command"
            ]
        );
    }

    #[test]
    fn windows_prefers_the_strict_reading_when_it_terminates() {
        // A doubled trailing backslash and an escaped-quote script: the strict
        // reading terminates, so it is the one used.
        assert_eq!(
            windows(r#"pwsh.exe -WorkingDirectory "C:\My Projects\\" -Command "echo \"a b\"""#),
            [
                "pwsh.exe",
                "-WorkingDirectory",
                r"C:\My Projects\",
                "-Command",
                r#"echo "a b""#
            ]
        );
    }

    #[test]
    fn windows_falls_back_to_whitespace_when_both_readings_leave_a_quote_open() {
        assert_eq!(
            windows(r#"pwsh.exe -WorkingDirectory "C:\My Projects\ -c"#),
            [
                "pwsh.exe",
                "-WorkingDirectory",
                r#""C:\My"#,
                r"Projects\",
                "-c"
            ]
        );
    }

    #[test]
    fn posix_still_processes_escapes() {
        assert_eq!(posix(r#"sh -c "a\"b""#), ["sh", "-c", "a\"b"]);
    }
}
