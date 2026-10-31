//! Parsers for the human-readable output that linters print by default.
//!
//! Builtins don't add flags that change what a tool prints, so these read each tool's default
//! output. Each parser recognizes the lines that start or locate a finding and ignores the rest
//! (source snippets, fix suggestions, summaries). Unrecognized lines become parse warnings only
//! when no finding was read at all ([`finish`]), so a tool that printed something else entirely,
//! such as a crash or a configuration error, is reported without flooding the warnings with
//! snippet lines. A tool's success summary is not a warning.

use super::{Diagnostic, ParseResult, Position, Range, Severity};
use regex::Regex;

fn finding(
    step: &str,
    tool: &str,
    severity: Severity,
    message: String,
    path: Option<String>,
    start: Option<(u64, u64)>,
) -> Diagnostic {
    Diagnostic {
        step: step.to_string(),
        tool: tool.to_string(),
        severity,
        message,
        path,
        range: start.map(|(line, column)| Range {
            start: Position {
                line: line.max(1),
                column: column.max(1),
            },
            end: None,
        }),
        rule: None,
        help_url: None,
        fix: None,
    }
}

fn number(value: &str) -> u64 {
    value.parse().unwrap_or(1)
}

/// How many unrecognized lines one parse reports.
const MAX_WARNINGS: usize = 20;

/// Records `line` as a parse warning; [`finish`] keeps them only when no finding was read.
fn unrecognized(parsed: &mut ParseResult, format: &str, line: &str) {
    let trimmed = line.trim();
    if trimmed.is_empty() || success_summary(trimmed) {
        return;
    }
    if parsed.warnings.len() < MAX_WARNINGS {
        parsed
            .warnings
            .push(format!("unrecognized {format} diagnostic: {line}"));
    } else if parsed.warnings.len() == MAX_WARNINGS {
        parsed
            .warnings
            .push(format!("more unrecognized {format} output left out"));
    }
}

/// What these tools print when they found nothing.
fn success_summary(line: &str) -> bool {
    static SUCCESS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^(?:All checks passed!?",
            r"|Checked \d+ files? in .*",
            r"|Found 0 \w+.*",
            r"|0 errors?, 0 warnings?, 0 informations?",
            r"|No fixes applied\.?",
            r"|No configuration file found\.?",
            r"|Success: .*",
            r")$"
        ))
        .expect("valid success regex")
    });
    SUCCESS.is_match(line)
}

/// Drops the parse warnings once any finding was read: the unrecognized lines were then the
/// excerpts and summaries around it.
pub(super) fn finish(mut parsed: ParseResult) -> ParseResult {
    if !parsed.diagnostics.is_empty() {
        parsed.warnings.retain(|w| w.starts_with(INCOMPLETE));
    }
    parsed
}

/// How a warning about a finding cut short starts: kept even when other findings were read.
const INCOMPLETE: &str = "incomplete ";

fn level(value: &str) -> Severity {
    match value {
        "warning" | "warn" | "!" | "⚠" => Severity::Warning,
        "note" | "info" | "information" | "i" | "ℹ" | "style" => Severity::Note,
        "help" => Severity::Help,
        _ => Severity::Error,
    }
}

/// rustc's human output, which cargo, clippy, ty, deno lint and ruff (0.12 and later) print:
/// a `severity[code]: message` (or ruff's `CODE message`) header, then ` --> path:line:col`.
///
/// A header with no location (`error: could not compile`, cargo's `generated 3 warnings`)
/// is not a finding. A `note:` or `help:` header inside an open finding is that finding's
/// child, as rustc prints a span note such as "the lint level is defined here". Without a
/// `[code]`, the rule is the lint the `= note: #[warn(lint)]` line names, and clippy's
/// `= help: for further information visit <url>` line becomes the help link.
pub(super) fn parse_rustc(step: &str, tool: &str, output: &str) -> ParseResult {
    let header = Regex::new(r"^(error|warning|note|help|info)(?:\[([^\]]+)\])?: (.+)$")
        .expect("valid rustc header regex");
    // ruff: `F401 [*] `os` imported but unused`
    let code_header =
        Regex::new(r"^([A-Z]+[0-9]+)(?: \[\*\])? (.+)$").expect("valid ruff header regex");
    let location = Regex::new(r"^\s*--> (.+):(\d+):(\d+)$").expect("valid location regex");
    let lint = Regex::new(r"^\s*= note: .*?`#\[(?:warn|deny|forbid|expect)\(([\w:]+)\)\]`")
        .expect("valid lint regex");
    let help =
        Regex::new(r"^\s*= help: for further information visit (\S+)$").expect("valid help regex");

    let mut parsed = ParseResult::default();
    // The header waiting for its ` --> ` line.
    let mut pending: Option<(Severity, Option<String>, String)> = None;
    // True while the last finding may take child notes.
    let mut open = false;
    for line in output.lines() {
        if line.trim().is_empty() {
            open = false;
            continue;
        }
        if let Some(c) = header.captures(line) {
            if open && matches!(&c[1], "note" | "help" | "info") {
                continue;
            }
            pending = Some((
                level(&c[1]),
                c.get(2).map(|m| m.as_str().into()),
                c[3].into(),
            ));
            open = false;
        } else if let Some(c) = code_header.captures(line) {
            pending = Some((Severity::Error, Some(c[1].into()), c[2].into()));
            open = false;
        } else if let Some(c) = location.captures(line) {
            let Some((severity, rule, message)) = pending.take() else {
                continue;
            };
            let mut diagnostic = finding(
                step,
                tool,
                severity,
                message,
                Some(c[1].to_string()),
                Some((number(&c[2]), number(&c[3]))),
            );
            diagnostic.rule = rule;
            parsed.diagnostics.push(diagnostic);
            open = true;
        } else if open {
            let Some(last) = parsed.diagnostics.last_mut() else {
                continue;
            };
            if let Some(c) = lint.captures(line) {
                last.rule.get_or_insert_with(|| c[1].to_string());
            } else if let Some(c) = help.captures(line) {
                last.help_url.get_or_insert_with(|| c[1].to_string());
            }
        } else if pending.is_none() {
            unrecognized(&mut parsed, "rustc", line);
        }
    }
    parsed
}

/// `rustfmt --check` and `cargo fmt --check`: a `Diff in path:line:` header per hunk (or
/// `Diff in path at line N:` from older releases), followed by the diff, which becomes the
/// message.
pub(super) fn parse_rustfmt(step: &str, tool: &str, output: &str) -> ParseResult {
    let header =
        Regex::new(r"^Diff in (.+?)(?::(\d+)| at line (\d+)):$").expect("valid rustfmt regex");
    let mut parsed = ParseResult::default();
    let mut open = false;
    for line in output.lines() {
        if let Some(c) = header.captures(line) {
            let row = c.get(2).or(c.get(3)).map_or(1, |m| number(m.as_str()));
            parsed.diagnostics.push(finding(
                step,
                tool,
                Severity::Warning,
                "not formatted; the formatter would change:".to_string(),
                Some(c[1].to_string()),
                Some((row, 1)),
            ));
            open = true;
        } else if open && (line.starts_with(['+', '-', ' ']) || line.is_empty()) {
            if let Some(last) = parsed.diagnostics.last_mut() {
                last.message.push('\n');
                last.message.push_str(line);
            }
        } else {
            open = false;
            unrecognized(&mut parsed, "rustfmt", line);
        }
    }
    for diagnostic in &mut parsed.diagnostics {
        diagnostic
            .message
            .truncate(diagnostic.message.trim_end().len());
    }
    parsed
}

/// ESLint's default `stylish` output, which stylelint and yamllint also print: a line naming
/// the file, then one `  line:col  severity  message  rule` row per finding, its columns
/// separated by two or more spaces. stylelint marks severity with `✖`/`⚠`, and yamllint puts
/// the rule in parentheses.
pub(super) fn parse_stylish(step: &str, tool: &str, output: &str) -> ParseResult {
    let row = Regex::new(r"^\s+(\d+):(\d+)\s+(error|warning|info|✖|⚠|ℹ|×)\s+(.+?)\s*$")
        .expect("valid stylish row regex");
    // The rule column: an ESLint or stylelint rule name (`no-unused-vars`, `react/jsx-key`,
    // `@typescript-eslint/no-explicit-any`) or yamllint's `(rule)`, after two or more spaces.
    let rule_column =
        Regex::new(r"^(.*?)\s{2,}(\(?@?[a-z0-9][a-z0-9-]*(?:/[a-z0-9][a-z0-9-]*)*\)?)$")
            .expect("valid rule column regex");
    let mut parsed = ParseResult::default();
    let mut file: Option<String> = None;
    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(c) = row.captures(line) {
            let Some(path) = file.clone() else {
                unrecognized(&mut parsed, "stylish", line);
                continue;
            };
            // Without a rule name at the end, the whole text is the message.
            let (message, rule) = match rule_column.captures(&c[4]) {
                Some(r) => (
                    r[1].to_string(),
                    Some(
                        r[2].trim_start_matches('(')
                            .trim_end_matches(')')
                            .to_string(),
                    ),
                ),
                None => (c[4].to_string(), None),
            };
            let mut diagnostic = finding(
                step,
                tool,
                level(&c[3]),
                message,
                Some(path),
                Some((number(&c[1]), number(&c[2]))),
            );
            diagnostic.rule = rule;
            parsed.diagnostics.push(diagnostic);
        } else if !line.starts_with(char::is_whitespace)
            && !line.starts_with(['✖', '⚠', '×', '✔', '!'])
        {
            // A file line, or an error the tool printed instead of findings.
            file = Some(line.trim_end().to_string());
            unrecognized(&mut parsed, "stylish", line);
        } else {
            unrecognized(&mut parsed, "stylish", line);
        }
    }
    parsed
}

/// tsc's output when it isn't writing to a terminal: `path(line,col): error TS2322: message`,
/// with any further lines of the message indented below it.
pub(super) fn parse_tsc(step: &str, tool: &str, output: &str) -> ParseResult {
    let entry = Regex::new(r"^(.+?)\((\d+),(\d+)\): (error|warning|message) (TS\d+): (.*)$")
        .expect("valid tsc regex");
    let mut parsed = ParseResult::default();
    let mut open = false;
    for line in output.lines() {
        if let Some(c) = entry.captures(line) {
            let mut diagnostic = finding(
                step,
                tool,
                level(&c[4]),
                c[6].to_string(),
                Some(c[1].to_string()),
                Some((number(&c[2]), number(&c[3]))),
            );
            diagnostic.rule = Some(c[5].to_string());
            parsed.diagnostics.push(diagnostic);
            open = true;
        } else if open && line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
            if let Some(last) = parsed.diagnostics.last_mut() {
                last.message.push('\n');
                last.message.push_str(line.trim());
            }
        } else {
            open = false;
            unrecognized(&mut parsed, "tsc", line);
        }
    }
    parsed
}

/// ShellCheck's default `tty` output: `In path line N:`, the source line, then a marker line
/// such as `     ^-- SC2086 (info): message` for each finding on it.
///
/// The marker is aligned to the source line with tabs expanded to eight columns, so the column
/// is found by walking the source line, matching what `shellcheck -f gcc` reports.
pub(super) fn parse_shellcheck(step: &str, tool: &str, output: &str) -> ParseResult {
    let location = Regex::new(r"^In (.+) line (\d+):$").expect("valid shellcheck location regex");
    let marker = Regex::new(r"^(\s*)\^[-^]*\s+SC(\d+) \((error|warning|info|style)\): (.*)$")
        .expect("valid shellcheck marker regex");
    let mut parsed = ParseResult::default();
    // The file, line and source text of the current `In ... line N:` block.
    let mut current: Option<(String, u64, Option<String>)> = None;
    for line in output.lines() {
        if let Some(c) = location.captures(line) {
            current = Some((c[1].to_string(), number(&c[2]), None));
        } else if let Some(c) = marker.captures(line) {
            let Some((path, row, source)) = &current else {
                unrecognized(&mut parsed, "shellcheck", line);
                continue;
            };
            let visual = c[1].chars().count();
            let column = source.as_deref().map_or(visual + 1, |source| {
                let mut width = 0;
                for (index, ch) in source.chars().enumerate() {
                    if width >= visual {
                        return index + 1;
                    }
                    width += if ch == '\t' { 8 - width % 8 } else { 1 };
                }
                source.chars().count() + 1
            });
            let mut diagnostic = finding(
                step,
                tool,
                level(&c[3]),
                c[4].to_string(),
                Some(path.clone()),
                Some((*row, column as u64)),
            );
            diagnostic.rule = Some(format!("SC{}", &c[2]));
            diagnostic.help_url = Some(format!("https://www.shellcheck.net/wiki/SC{}", &c[2]));
            parsed.diagnostics.push(diagnostic);
        } else if let Some((_, _, source @ None)) = &mut current {
            *source = Some(line.to_string());
        } else {
            unrecognized(&mut parsed, "shellcheck", line);
        }
    }
    parsed
}

/// pyright's default output: the file's path, then `  path:line:col - error: message` per
/// finding, with any further lines of the message indented below it (with no-break spaces)
/// and the rule in parentheses at the end of the last one.
pub(super) fn parse_pyright(step: &str, tool: &str, output: &str) -> ParseResult {
    let entry = Regex::new(r"^\s+(.+?):(\d+):(\d+) - (error|warning|information): (.*)$")
        .expect("valid pyright regex");
    let rule = Regex::new(r"\s+\((report\w+)\)$").expect("valid pyright rule regex");
    let mut parsed = ParseResult::default();
    let mut open = false;
    for line in output.lines() {
        if let Some(c) = entry.captures(line) {
            parsed.diagnostics.push(finding(
                step,
                tool,
                level(&c[4]),
                c[5].to_string(),
                Some(c[1].to_string()),
                Some((number(&c[2]), number(&c[3]))),
            ));
            open = true;
        } else if open && line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
            if let Some(last) = parsed.diagnostics.last_mut() {
                last.message.push('\n');
                last.message.push_str(line.trim());
            }
        } else {
            open = false;
            unrecognized(&mut parsed, "pyright", line);
        }
    }
    for diagnostic in &mut parsed.diagnostics {
        if let Some(c) = rule.captures(&diagnostic.message) {
            diagnostic.rule = Some(c[1].to_string());
            let end = c.get(0).map_or(diagnostic.message.len(), |m| m.start());
            diagnostic.message.truncate(end);
        }
    }
    parsed
}

/// Biome's default output: a `path:line:col category  FIXABLE  ━━━` header per finding (with
/// no position for whole-file findings such as `format`), then the message after a `×`, `!` or
/// `i` marker that gives its severity.
pub(super) fn parse_biome(step: &str, tool: &str, output: &str) -> ParseResult {
    // The path may hold spaces: the category is the last word before the flags and the rule.
    let header = Regex::new(r"^(\S.*?)(?::(\d+):(\d+))? ([^\s━]+)(?:\s+[A-Z]+)*\s+━{3,}$")
        .expect("valid biome header regex");
    let message = Regex::new(r"^\s+([×✖!⚠iℹ]) (.+)$").expect("valid biome message regex");
    let mut parsed = ParseResult::default();
    // A header waiting for its message, and the header line, which is reported if no message
    // follows.
    let mut pending: Option<(Diagnostic, String)> = None;
    let incomplete = |parsed: &mut ParseResult, header: String| {
        parsed.warnings.push(format!(
            "{INCOMPLETE}biome finding with no message: {header}"
        ));
    };
    for line in output.lines() {
        if let Some(c) = header.captures(line) {
            if let Some((_, previous)) = pending.take() {
                incomplete(&mut parsed, previous);
            }
            let start = c
                .get(2)
                .zip(c.get(3))
                .map(|(row, column)| (number(row.as_str()), number(column.as_str())));
            let mut diagnostic = finding(
                step,
                tool,
                Severity::Error,
                String::new(),
                Some(c[1].to_string()),
                start,
            );
            diagnostic.rule = Some(c[4].to_string());
            pending = Some((diagnostic, line.to_string()));
        } else if let Some(c) = message.captures(line)
            && let Some((mut diagnostic, _)) = pending.take()
        {
            diagnostic.severity = level(&c[1]);
            diagnostic.message = c[2].trim_end().to_string();
            parsed.diagnostics.push(diagnostic);
        } else if !line.starts_with(char::is_whitespace) {
            unrecognized(&mut parsed, "biome", line);
        }
    }
    if let Some((_, header)) = pending {
        incomplete(&mut parsed, header);
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::super::{DiagnosticFormat, parse};
    use super::*;

    fn located(d: &Diagnostic) -> (Option<&str>, u64, u64) {
        let start = &d.range.as_ref().expect("range").start;
        (d.path.as_deref(), start.line, start.column)
    }

    #[test]
    fn rustc_reads_clippy_warnings_with_lint_and_help_link() {
        let output = r#"warning: unused variable: `x`
 --> src/main.rs:2:9
  |
2 |     let x = 5;
  |         ^ help: if this is intentional, prefix it with an underscore: `_x`
  |
  = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

warning: length comparison to zero
 --> src/main.rs:4:8
  |
4 |     if v.len() == 0 {}
  |        ^^^^^^^^^^^^ help: using `is_empty` is clearer and more explicit: `v.is_empty()`
  |
  = help: for further information visit https://rust-lang.github.io/rust-clippy/beta/index.html#len_zero
  = note: `#[warn(clippy::len_zero)]` on by default

warning: `t` (bin "t") generated 2 warnings
"#;
        let parsed = parse(DiagnosticFormat::Rustc, "clippy", "clippy", output);
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.diagnostics.len(), 2);
        let first = &parsed.diagnostics[0];
        assert_eq!(located(first), (Some("src/main.rs"), 2, 9));
        assert_eq!(first.severity, Severity::Warning);
        assert_eq!(first.message, "unused variable: `x`");
        assert_eq!(first.rule.as_deref(), Some("unused_variables"));
        let second = &parsed.diagnostics[1];
        assert_eq!(second.rule.as_deref(), Some("clippy::len_zero"));
        assert_eq!(
            second.help_url.as_deref(),
            Some("https://rust-lang.github.io/rust-clippy/beta/index.html#len_zero")
        );
    }

    #[test]
    fn rustc_reads_error_codes_and_skips_unlocated_errors_and_child_notes() {
        let output = r#"error[E0308]: mismatched types
 --> src/main.rs:5:17
  |
5 |     let y: u8 = "a";
  |            --   ^^^ expected `u8`, found `&str`
note: the lint level is defined here
 --> src/main.rs:1:9

For more information about this error, try `rustc --explain E0308`.
error: could not compile `t` (bin "t") due to 1 previous error
"#;
        let parsed = parse(DiagnosticFormat::Rustc, "check", "cargo", output);
        assert_eq!(parsed.diagnostics.len(), 1);
        let d = &parsed.diagnostics[0];
        assert_eq!(located(d), (Some("src/main.rs"), 5, 17));
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.rule.as_deref(), Some("E0308"));
    }

    #[test]
    fn rustc_reads_ty_and_ruff() {
        let ty = r#"error[invalid-assignment]: Object of type `Literal[""]` is not assignable to `int`
 --> t.py:2:10
  |
2 | x: int = ""
  |    ---   ^^ Incompatible value of type `Literal[""]`
  |
info: rule `invalid-assignment` is enabled by default

Found 1 diagnostic
"#;
        let parsed = parse(DiagnosticFormat::Rustc, "ty", "ty", ty);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(located(&parsed.diagnostics[0]), (Some("t.py"), 2, 10));
        assert_eq!(
            parsed.diagnostics[0].rule.as_deref(),
            Some("invalid-assignment")
        );

        let ruff = r#"F401 [*] `os` imported but unused
 --> t.py:1:8
  |
1 | import os
  |        ^^
help: Remove unused import: `os`
  |
  - import os
1 | x: int = ""
  |

E741 Ambiguous variable name: `l`
 --> t.py:3:1

Found 2 errors.
[*] 1 fixable with `--fix`.
"#;
        let parsed = parse(DiagnosticFormat::Rustc, "ruff", "ruff", ruff);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(parsed.diagnostics[0].rule.as_deref(), Some("F401"));
        assert_eq!(parsed.diagnostics[0].message, "`os` imported but unused");
        assert_eq!(located(&parsed.diagnostics[1]), (Some("t.py"), 3, 1));
        assert_eq!(parsed.diagnostics[1].rule.as_deref(), Some("E741"));
    }

    #[test]
    fn rustc_reads_colored_output() {
        let output = "\u{1b}[0m\u{1b}[1m\u{1b}[33mwarning\u{1b}[0m\u{1b}[1m: unused variable: `x`\u{1b}[0m\n\u{1b}[0m \u{1b}[1m\u{1b}[94m--> \u{1b}[0msrc/main.rs:2:9\n";
        let parsed = parse(DiagnosticFormat::Rustc, "check", "cargo", output);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(located(&parsed.diagnostics[0]), (Some("src/main.rs"), 2, 9));
    }

    #[test]
    fn rustc_reports_unrecognized_output() {
        let parsed = parse(DiagnosticFormat::Rustc, "s", "t", "Segmentation fault");
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.warnings.len(), 1);
    }

    #[test]
    fn rustfmt_reads_each_hunk_with_its_diff() {
        let output = "Diff in /repo/src/main.rs:1:\n-fn main(){\n+fn main() {\n     let x = 5;\nDiff in /repo/src/lib.rs at line 7:\n-a\n+b\n\n";
        let parsed = parse(DiagnosticFormat::Rustfmt, "rustfmt", "rustfmt", output);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(
            located(&parsed.diagnostics[0]),
            (Some("/repo/src/main.rs"), 1, 1)
        );
        assert!(
            parsed.diagnostics[0]
                .message
                .ends_with("\n-fn main(){\n+fn main() {\n     let x = 5;")
        );
        assert_eq!(
            located(&parsed.diagnostics[1]),
            (Some("/repo/src/lib.rs"), 7, 1)
        );
        assert_eq!(parsed.diagnostics[1].severity, Severity::Warning);
    }

    #[test]
    fn stylish_reads_eslint_stylelint_and_yamllint() {
        let eslint = "\n/repo/t.js\n  1:7  error    'x' is assigned a value but never used  no-unused-vars\n  2:1  warning  Unexpected 'debugger' statement         no-debugger\n  3:1  error    Parsing error: Unexpected token\n\n✖ 3 problems (2 errors, 1 warning)\n";
        let parsed = parse(DiagnosticFormat::Stylish, "eslint", "eslint", eslint);
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.diagnostics.len(), 3);
        assert_eq!(located(&parsed.diagnostics[0]), (Some("/repo/t.js"), 1, 7));
        assert_eq!(
            parsed.diagnostics[0].message,
            "'x' is assigned a value but never used"
        );
        assert_eq!(
            parsed.diagnostics[0].rule.as_deref(),
            Some("no-unused-vars")
        );
        assert_eq!(parsed.diagnostics[1].severity, Severity::Warning);
        assert_eq!(parsed.diagnostics[2].rule, None);

        let stylelint = "\nt.css\n  1:12  ✖  Invalid hex color \"#fffff\"  color-no-invalid-hex\n\n✖ 1 problem (1 error, 0 warnings)\n";
        let parsed = parse(
            DiagnosticFormat::Stylish,
            "stylelint",
            "stylelint",
            stylelint,
        );
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(parsed.diagnostics[0].severity, Severity::Error);
        assert_eq!(
            parsed.diagnostics[0].rule.as_deref(),
            Some("color-no-invalid-hex")
        );

        let yamllint = "t.yaml\n  1:1       warning  missing document start \"---\"  (document-start)\n  2:1       error    duplication of key \"a\" in mapping  (key-duplicates)\n";
        let parsed = parse(DiagnosticFormat::Stylish, "yamllint", "yamllint", yamllint);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(located(&parsed.diagnostics[1]), (Some("t.yaml"), 2, 1));
        assert_eq!(
            parsed.diagnostics[1].rule.as_deref(),
            Some("key-duplicates")
        );
    }

    #[test]
    fn tsc_reads_findings_and_continuation_lines() {
        let output = "src/a.ts(1,5): error TS2322: Type 'string' is not assignable to type 'number'.\nsrc/b.ts(3,1): error TS2345: Argument of type 'X' is not assignable.\n  Property 'y' is missing.\n\nFound 2 errors in 2 files.\n";
        let parsed = parse(DiagnosticFormat::Tsc, "tsc", "tsc", output);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(located(&parsed.diagnostics[0]), (Some("src/a.ts"), 1, 5));
        assert_eq!(parsed.diagnostics[0].rule.as_deref(), Some("TS2322"));
        assert_eq!(
            parsed.diagnostics[1].message,
            "Argument of type 'X' is not assignable.\nProperty 'y' is missing."
        );
    }

    #[test]
    fn shellcheck_reads_markers_and_maps_tab_columns() {
        let output = "\nIn t.sh line 2:\necho $1\n     ^-- SC2086 (info): Double quote to prevent globbing and word splitting.\n\nDid you mean:\necho \"$1\"\n\n\nIn t.sh line 3:\n\tcd foo\n        ^----^ SC2164 (warning): Use 'cd ... || exit' in case cd fails.\n\nFor more information:\n  https://www.shellcheck.net/wiki/SC2164 -- Use 'cd ... || exit' ...\n";
        let parsed = parse(
            DiagnosticFormat::Shellcheck,
            "shellcheck",
            "shellcheck",
            output,
        );
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(located(&parsed.diagnostics[0]), (Some("t.sh"), 2, 6));
        assert_eq!(parsed.diagnostics[0].severity, Severity::Note);
        assert_eq!(parsed.diagnostics[0].rule.as_deref(), Some("SC2086"));
        // `\tcd` puts the marker at screen column 8, which is the second character.
        assert_eq!(located(&parsed.diagnostics[1]), (Some("t.sh"), 3, 2));
        assert_eq!(parsed.diagnostics[1].severity, Severity::Warning);
        assert_eq!(
            parsed.diagnostics[1].help_url.as_deref(),
            Some("https://www.shellcheck.net/wiki/SC2164")
        );
    }

    #[test]
    fn pyright_reads_multiline_messages_and_trailing_rule() {
        let output = "/repo/t.py\n  /repo/t.py:2:10 - error: Type \"Literal['']\" is not assignable to declared type \"int\"\n\u{a0}\u{a0}\u{a0}\u{a0}\"Literal['']\" is not assignable to \"int\" (reportAssignmentType)\n  /repo/t.py:4:1 - warning: Import \"x\" could not be resolved (reportMissingImports)\n1 error, 1 warning, 0 informations\n";
        let parsed = parse(DiagnosticFormat::Pyright, "pyright", "pyright", output);
        assert_eq!(parsed.diagnostics.len(), 2);
        let first = &parsed.diagnostics[0];
        assert_eq!(located(first), (Some("/repo/t.py"), 2, 10));
        assert_eq!(first.rule.as_deref(), Some("reportAssignmentType"));
        assert_eq!(
            first.message,
            "Type \"Literal['']\" is not assignable to declared type \"int\"\n\"Literal['']\" is not assignable to \"int\""
        );
        assert_eq!(parsed.diagnostics[1].severity, Severity::Warning);
        assert_eq!(
            parsed.diagnostics[1].rule.as_deref(),
            Some("reportMissingImports")
        );
    }

    #[test]
    fn biome_reads_lint_and_format_findings() {
        let output = "t.js:1:7 lint/correctness/noUnusedVariables  FIXABLE  ━━━━━━━━━━━━━━━━━━━━\n\n  ! This variable x is unused.\n  \n  > 1 │ const x = 1;\n  \n  i Unused variables are often the result of typos.\n\nt.js:2:1 lint/suspicious/noDebugger  FIXABLE  ━━━━━━━━━━━━━━━━━━━━\n\n  × This is an unexpected use of the debugger statement.\n\nt.js format ━━━━━━━━━━━━━━━━━━━━\n\n  × File content differs from formatting output\n\nChecked 1 file in 6ms. No fixes applied.\nFound 2 errors.\ncheck ━━━━━━━━━━━━━━━━━━━━\n\n  × Some errors were emitted while running checks.\n";
        let parsed = parse(DiagnosticFormat::Biome, "biome", "biome", output);
        assert_eq!(parsed.diagnostics.len(), 3);
        let first = &parsed.diagnostics[0];
        assert_eq!(located(first), (Some("t.js"), 1, 7));
        assert_eq!(first.severity, Severity::Warning);
        assert_eq!(first.message, "This variable x is unused.");
        assert_eq!(
            first.rule.as_deref(),
            Some("lint/correctness/noUnusedVariables")
        );
        assert_eq!(parsed.diagnostics[1].severity, Severity::Error);
        let format = &parsed.diagnostics[2];
        assert_eq!(format.path.as_deref(), Some("t.js"));
        assert_eq!(format.range, None);
        assert_eq!(format.rule.as_deref(), Some("format"));
    }

    #[test]
    fn unreadable_output_is_a_warning_and_success_is_not() {
        for format in [
            DiagnosticFormat::Rustc,
            DiagnosticFormat::Rustfmt,
            DiagnosticFormat::Stylish,
            DiagnosticFormat::Tsc,
            DiagnosticFormat::Shellcheck,
            DiagnosticFormat::Pyright,
            DiagnosticFormat::Biome,
        ] {
            let parsed = parse(format, "s", "t", "Segmentation fault (core dumped)\n");
            assert!(parsed.diagnostics.is_empty(), "{format:?}");
            assert_eq!(
                parsed.warnings.len(),
                1,
                "{format:?}: {:?}",
                parsed.warnings
            );
        }
        for (format, success) in [
            (DiagnosticFormat::Rustc, "All checks passed!\n"),
            (
                DiagnosticFormat::Pyright,
                "0 errors, 0 warnings, 0 informations \n",
            ),
            (
                DiagnosticFormat::Biome,
                "Checked 1 file in 3ms. No fixes applied.\n",
            ),
            (DiagnosticFormat::Tsc, "Found 0 errors.\n"),
        ] {
            let parsed = parse(format, "s", "t", success);
            assert!(
                parsed.warnings.is_empty(),
                "{format:?}: {:?}",
                parsed.warnings
            );
        }
        // An ESLint configuration error prints only unindented lines.
        let parsed = parse(
            DiagnosticFormat::Stylish,
            "eslint",
            "eslint",
            "Oops! Something went wrong! :(\nESLint couldn't find an eslint.config.js file.\n",
        );
        assert_eq!(parsed.warnings.len(), 2, "{:?}", parsed.warnings);
    }

    #[test]
    fn stylish_keeps_spaced_messages_without_a_rule_whole() {
        let output = "a.js\n  1:5  error  Parsing error:  Unexpected token\n  2:1  error  Unexpected var  @typescript-eslint/no-var-requires\n";
        let parsed = parse(DiagnosticFormat::Stylish, "eslint", "eslint", output);
        assert_eq!(
            parsed.diagnostics[0].message,
            "Parsing error:  Unexpected token"
        );
        assert_eq!(parsed.diagnostics[0].rule, None);
        assert_eq!(parsed.diagnostics[1].message, "Unexpected var");
        assert_eq!(
            parsed.diagnostics[1].rule.as_deref(),
            Some("@typescript-eslint/no-var-requires")
        );
    }

    #[test]
    fn shellcheck_reports_lines_inside_a_block_it_cannot_read() {
        let output = "\nIn t.sh line 2:\necho $1\nshellcheck: internal error\n";
        let parsed = parse(
            DiagnosticFormat::Shellcheck,
            "shellcheck",
            "shellcheck",
            output,
        );
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.warnings.len(), 1, "{:?}", parsed.warnings);
    }

    #[test]
    fn biome_reports_a_header_with_no_message_instead_of_a_finding() {
        let output = "a.js:1:1 lint/style/useConst  FIXABLE  ━━━━━━━━━━\n\nb.js:2:1 lint/suspicious/noDebugger  ━━━━━━━━━━\n\n  × This is an unexpected use of the debugger statement.\n\nc.js:3:1 lint/style/useConst  ━━━━━━━━━━\n";
        let parsed = parse(DiagnosticFormat::Biome, "biome", "biome", output);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(parsed.diagnostics[0].path.as_deref(), Some("b.js"));
        assert_eq!(parsed.warnings.len(), 2, "{:?}", parsed.warnings);
        assert!(
            parsed.warnings[0].starts_with("incomplete biome finding with no message: a.js:1:1")
        );
    }

    #[test]
    fn biome_reads_paths_with_spaces() {
        let output = "src/my file.ts:2:1 lint/suspicious/noDebugger  FIXABLE  ━━━━━━━━━━\n\n  × This is an unexpected use of the debugger statement.\n\nsrc/my file.ts format ━━━━━━━━━━\n\n  × File content differs from formatting output\n";
        let parsed = parse(DiagnosticFormat::Biome, "biome", "biome", output);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(
            located(&parsed.diagnostics[0]),
            (Some("src/my file.ts"), 2, 1)
        );
        assert_eq!(
            parsed.diagnostics[1].path.as_deref(),
            Some("src/my file.ts")
        );
        assert_eq!(parsed.diagnostics[1].rule.as_deref(), Some("format"));
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    }

    #[test]
    fn gcc_paren_reads_golangci_lint_and_staticcheck() {
        let golangci = "main.go:1: : # example.com/m\nmain.go:6:2: ineffectual assignment to x (ineffassign)\n\tx := 1\n\t^\nmain.go:11:2: SA9003: empty branch (staticcheck)\n\tif s == nil {\n\t^\n2 issues:\n* ineffassign: 1\n* staticcheck: 1\n";
        let parsed = parse(
            DiagnosticFormat::GccParen,
            "golangci",
            "golangci-lint",
            golangci,
        );
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(located(&parsed.diagnostics[0]), (Some("main.go"), 6, 2));
        assert_eq!(parsed.diagnostics[0].message, "ineffectual assignment to x");
        assert_eq!(parsed.diagnostics[0].rule.as_deref(), Some("ineffassign"));
        assert_eq!(parsed.diagnostics[1].message, "SA9003: empty branch");

        let staticcheck = "main.go:11:5: should omit nil check; len() for nil slices is defined as zero (S1009)\n";
        let parsed = parse(
            DiagnosticFormat::GccParen,
            "staticcheck",
            "staticcheck",
            staticcheck,
        );
        assert_eq!(parsed.diagnostics[0].rule.as_deref(), Some("S1009"));
    }
}
