use crate::{Result, step::DiagnosticFormat};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub line: u64,
    pub column: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<Position>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DiagnosticFix {
    pub replacement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Diagnostic {
    pub step: String,
    pub tool: String,
    pub severity: Severity,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<DiagnosticFix>,
}

#[derive(Debug, Default)]
pub struct ParseResult {
    pub diagnostics: Vec<Diagnostic>,
    pub warnings: Vec<String>,
}

pub fn parse(format: DiagnosticFormat, step: &str, tool: &str, output: &str) -> ParseResult {
    let mut result = match format {
        DiagnosticFormat::Sarif => parse_sarif(step, tool, output),
        DiagnosticFormat::CargoJson => parse_cargo(step, tool, output),
        DiagnosticFormat::EslintJson => parse_eslint(step, tool, output),
        DiagnosticFormat::Gcc => parse_gcc(step, tool, output),
    };
    let mut seen = IndexSet::new();
    result
        .diagnostics
        .retain(|diagnostic| seen.insert(diagnostic.clone()));
    result
}

/// Parse several commands' output, each run in its own directory, into one result.
///
/// Tools print paths relative to the directory they ran in, so each segment is
/// parsed on its own and its paths are rebased onto the repository root with
/// [`rebase_paths`].
pub fn parse_segments<'a>(
    format: DiagnosticFormat,
    step: &str,
    tool: &str,
    segments: impl IntoIterator<Item = (Option<&'a str>, &'a str)>,
) -> ParseResult {
    let mut merged = ParseResult::default();
    for (dir, output) in segments {
        let mut parsed = parse(format, step, tool, output);
        if let Some(dir) = dir {
            rebase_paths(&mut parsed.diagnostics, dir);
        }
        merged.diagnostics.append(&mut parsed.diagnostics);
        merged.warnings.append(&mut parsed.warnings);
    }
    let mut seen = IndexSet::new();
    merged
        .diagnostics
        .retain(|diagnostic| seen.insert(diagnostic.clone()));
    merged
}

/// Make diagnostic paths relative to the repository root.
///
/// `dir` is the directory, relative to the repository root, the tool ran in.
/// Relative paths are prefixed with it and lexically normalized (`./` and
/// `..` segments resolved, `\` treated as a separator). Absolute paths, Windows
/// drive paths and URIs with a scheme are left alone.
pub fn rebase_paths(diagnostics: &mut [Diagnostic], dir: &str) {
    let dir = dir.replace('\\', "/");
    if Path::new(&dir).is_absolute() || dir.starts_with('/') {
        return;
    }
    let dir = normalize(&dir);
    if dir.is_empty() {
        return;
    }
    for diagnostic in diagnostics {
        if let Some(path) = &mut diagnostic.path {
            *path = rebase(&dir, path);
        }
        if let Some(path) = diagnostic.fix.as_mut().and_then(|fix| fix.path.as_mut()) {
            *path = rebase(&dir, path);
        }
    }
}

fn is_absolute_like(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || path.starts_with('\\')
        // `C:\x` or `C:/x`
        || (bytes.len() > 1 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
            && matches!(bytes.get(2), Some(b'/' | b'\\')))
        // URIs such as `file:///x`
        || path.contains("://")
}

fn rebase(dir: &str, path: &str) -> String {
    if path.is_empty() || is_absolute_like(path) {
        return path.to_string();
    }
    normalize(&format!("{dir}/{}", path.replace('\\', "/")))
}

/// Lexically resolve `.` and `..`; a leading `..` that escapes the root is kept.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else {
                    parts.push("..");
                }
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

fn severity(value: &str) -> Severity {
    match value.to_ascii_lowercase().as_str() {
        "warning" | "warn" | "1" => Severity::Warning,
        "note" | "info" | "3" => Severity::Note,
        "help" | "4" => Severity::Help,
        _ => Severity::Error,
    }
}

fn position(line: Option<u64>, column: Option<u64>) -> Option<Position> {
    line.map(|line| Position {
        line,
        column: column.unwrap_or(1),
    })
}

fn parse_cargo(step: &str, tool: &str, output: &str) -> ParseResult {
    let mut parsed = ParseResult::default();
    for (index, line) in output.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => {
                parsed
                    .warnings
                    .push(format!("line {} is not Cargo JSON: {err}", index + 1));
                continue;
            }
        };
        if value.get("reason").and_then(Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let span = message
            .get("spans")
            .and_then(Value::as_array)
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|span| span.get("is_primary").and_then(Value::as_bool) == Some(true))
                    .or_else(|| spans.first())
            });
        parsed.diagnostics.push(Diagnostic {
            step: step.to_string(),
            tool: tool.to_string(),
            severity: severity(
                message
                    .get("level")
                    .and_then(Value::as_str)
                    .unwrap_or("error"),
            ),
            message: message
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("compiler diagnostic")
                .to_string(),
            path: span
                .and_then(|span| span.get("file_name"))
                .and_then(Value::as_str)
                .map(str::to_string),
            range: span.and_then(|span| {
                Some(Range {
                    start: position(
                        span.get("line_start").and_then(Value::as_u64),
                        span.get("column_start").and_then(Value::as_u64),
                    )?,
                    end: position(
                        span.get("line_end").and_then(Value::as_u64),
                        span.get("column_end").and_then(Value::as_u64),
                    ),
                })
            }),
            rule: message
                .get("code")
                .and_then(|code| code.get("code"))
                .and_then(Value::as_str)
                .map(str::to_string),
            help_url: None,
            fix: None,
        });
    }
    parsed
}

fn parse_eslint(step: &str, tool: &str, output: &str) -> ParseResult {
    let mut parsed = ParseResult::default();
    let files: Vec<Value> = match serde_json::from_str(output) {
        Ok(files) => files,
        Err(err) => {
            parsed.warnings.push(format!("invalid ESLint JSON: {err}"));
            return parsed;
        }
    };
    for file in files {
        let path = file
            .get("filePath")
            .and_then(Value::as_str)
            .map(str::to_string);
        for message in file
            .get("messages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            parsed.diagnostics.push(Diagnostic {
                step: step.to_string(),
                tool: tool.to_string(),
                severity: severity(
                    &message
                        .get("severity")
                        .and_then(Value::as_u64)
                        .unwrap_or(2)
                        .to_string(),
                ),
                message: message
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("ESLint diagnostic")
                    .to_string(),
                path: path.clone(),
                range: position(
                    message.get("line").and_then(Value::as_u64),
                    message.get("column").and_then(Value::as_u64),
                )
                .map(|start| Range {
                    start,
                    end: position(
                        message.get("endLine").and_then(Value::as_u64),
                        message.get("endColumn").and_then(Value::as_u64),
                    ),
                }),
                rule: message
                    .get("ruleId")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                help_url: None,
                fix: message.get("fix").map(|fix| DiagnosticFix {
                    replacement: fix
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    path: path.clone(),
                    range: None,
                }),
            });
        }
    }
    parsed
}

/// Parses `path:line[:column]: [severity:] message [rule]` lines.
///
/// Beyond the classic GCC shape, this is deliberately tolerant of the way other
/// tools print the same thing, without reading anything ambiguous:
///
/// - the column is optional (`path:line: message`, as mypy and buildifier print);
/// - go vet's `vet: ` prefix is ignored, and `# package` header lines (also
///   `path:1: : # package`, as golangci-lint prints) are skipped, but not
///   preprocessor lines such as `#define X`;
/// - a message continues over the following lines only until a blank line or a
///   recognizable summary line, so trailing summaries never reach the last finding.
fn parse_gcc(step: &str, tool: &str, output: &str) -> ParseResult {
    const RULE: &str = r"(?:\s+\[([^\]]+)\](?:\s+\[\d+\])?)?$";
    let with_column = regex::Regex::new(&format!(
        r"^(.*?):(\d+):(\d+):\s*(?:(error|warning|note|help):\s*)?(.*?){RULE}"
    ))
    .expect("valid GCC diagnostic regex");
    // Without a column the finding is easy to confuse with source text that clang-format
    // and gcc print under a finding, so the path must be one token (no whitespace,
    // which also keeps indented sub-locations continuation lines), must not be a URL
    // (`http://x:80: y`), and must not start a comment (`//note:12: text`).
    let without_column = regex::Regex::new(&format!(
        r"^(\S+?):(\d+):\s+(?:(error|warning|note|help):\s*)?([^\s:].*?){RULE}"
    ))
    .expect("valid column-less diagnostic regex");
    // Only go vet's prefix is stripped: any other `word: ` may be the start of a path.
    let tool_prefix = regex::Regex::new(r"^vet: (\S)").expect("valid prefix regex");
    // `# example.com/m`, `# [example.com/m]`, `# example.com/m [example.com/m.test]`
    let header = regex::Regex::new(r"^(?:\S+:\d+:\s*:\s*)?# \[?([\w.~/-]+)\]?(?: \[[\w.~/-]+\])?$")
        .expect("valid header regex");
    let is_header = |line: &str| {
        header.captures(line).is_some_and(|c| {
            !matches!(
                &c[1],
                "include"
                    | "include_next"
                    | "import"
                    | "define"
                    | "undef"
                    | "if"
                    | "ifdef"
                    | "ifndef"
                    | "elif"
                    | "else"
                    | "endif"
                    | "pragma"
                    | "error"
                    | "warning"
                    | "line"
            )
        })
    };
    let summary = regex::Regex::new(concat!(
        r"^(?:",
        // `Found 3 errors in 2 files`, `3 issues:`, `1 error generated.`
        r"(?:Found )?\d+ (?:errors?|issues?|warnings?|problems?)\b",
        // cpplint: `Done processing x.cc`, `Total errors found: 4`
        r"|Done processing\b|Total (?:errors?|issues?|warnings?)\b.*: \d+$",
        // sorbet: `Errors: 2`
        r"|Errors: \d+$",
        // buildifier: `BUILD.bazel # reformat`
        r"|\S.* # reformat$",
        // timestamped log lines, such as ktlint's WARN line
        r"|\d{1,2}:\d{2}:\d{2}[.,]\d+\s",
        r")"
    ))
    .expect("valid summary regex");

    let locate = |line: &str| -> Option<Diagnostic> {
        let line = match tool_prefix.captures(line) {
            Some(prefix) => &line[prefix.get(1).map_or(0, |m| m.start())..],
            None => line,
        };
        let (path, row, column, level, message, rule) = if let Some(c) = with_column.captures(line)
        {
            (
                c[1].to_string(),
                c[2].to_string(),
                c[3].parse().unwrap_or(1),
                c.get(4),
                c[5].to_string(),
                c.get(6),
            )
        } else {
            let c = without_column.captures(line)?;
            let path = &c[1];
            if path.contains("://") || path.starts_with("//") || path.starts_with("/*") {
                return None;
            }
            (
                c[1].to_string(),
                c[2].to_string(),
                1,
                c.get(3),
                c[4].to_string(),
                c.get(5),
            )
        };
        // `[*]` marks an auto-fixable finding (rumdl); it is not a rule name.
        let rule = rule
            .map(|value| value.as_str())
            .filter(|value| *value != "*")
            .map(str::to_string);
        Some(Diagnostic {
            step: step.to_string(),
            tool: tool.to_string(),
            severity: severity(level.map_or("error", |value| value.as_str())),
            message,
            path: Some(path),
            range: Some(Range {
                start: Position {
                    // SARIF positions are 1-based; cpplint prints line 0 for file-level findings.
                    line: row.parse().unwrap_or(1).max(1),
                    column: u64::max(column, 1),
                },
                end: None,
            }),
            rule,
            help_url: None,
            fix: None,
        })
    };

    let mut parsed = ParseResult::default();
    // True while the last finding may still take continuation lines.
    let mut open = false;
    for line in output.lines() {
        if line.trim().is_empty() {
            open = false;
        } else if let Some(diagnostic) = locate(line) {
            parsed.diagnostics.push(diagnostic);
            open = true;
        } else if is_header(line) || summary.is_match(line) {
            open = false;
        } else if open {
            if let Some(previous) = parsed.diagnostics.last_mut() {
                previous.message.push('\n');
                previous.message.push_str(line);
            }
        } else if parsed.diagnostics.is_empty() {
            parsed
                .warnings
                .push(format!("unrecognized GCC diagnostic: {line}"));
        }
    }
    parsed
}

fn parse_sarif(step: &str, tool: &str, output: &str) -> ParseResult {
    let mut parsed = ParseResult::default();
    let value: Value = match serde_json::from_str(output) {
        Ok(value) => value,
        Err(err) => {
            parsed.warnings.push(format!("invalid SARIF: {err}"));
            return parsed;
        }
    };
    for run in value
        .get("runs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for result in run
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let physical = result.pointer("/locations/0/physicalLocation");
            let region = physical.and_then(|location| location.get("region"));
            parsed.diagnostics.push(Diagnostic {
                step: step.to_string(),
                tool: tool.to_string(),
                severity: severity(
                    result
                        .get("level")
                        .and_then(Value::as_str)
                        .unwrap_or("error"),
                ),
                message: result
                    .pointer("/message/text")
                    .and_then(Value::as_str)
                    .unwrap_or("SARIF diagnostic")
                    .to_string(),
                path: physical
                    .and_then(|location| location.pointer("/artifactLocation/uri"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                range: region.and_then(|region| {
                    Some(Range {
                        start: position(
                            region.get("startLine").and_then(Value::as_u64),
                            region.get("startColumn").and_then(Value::as_u64),
                        )?,
                        end: position(
                            region.get("endLine").and_then(Value::as_u64),
                            region.get("endColumn").and_then(Value::as_u64),
                        ),
                    })
                }),
                rule: result
                    .get("ruleId")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                help_url: result
                    .get("helpUri")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                fix: None,
            });
        }
    }
    parsed
}

pub fn write_sarif(path: &Path, diagnostics: &[Diagnostic]) -> Result<()> {
    let results = diagnostics
        .iter()
        .map(|diagnostic| {
            let location = diagnostic.path.as_ref().map(|path| {
                let mut physical_location = serde_json::json!({
                    "artifactLocation": {"uri": path},
                });
                if let Some(range) = &diagnostic.range {
                    let mut region = serde_json::json!({
                        "startLine": range.start.line,
                        "startColumn": range.start.column,
                    });
                    if let Some(end) = &range.end {
                        region["endLine"] = serde_json::json!(end.line);
                        region["endColumn"] = serde_json::json!(end.column);
                    }
                    physical_location["region"] = region;
                }
                serde_json::json!({
                    "physicalLocation": physical_location,
                })
            });
            let mut result = serde_json::json!({
                "level": match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Note | Severity::Help => "note",
                },
                "message": {"text": diagnostic.message},
                "locations": location.into_iter().collect::<Vec<_>>(),
                "properties": {"step": diagnostic.step, "tool": diagnostic.tool},
            });
            if let Some(rule) = &diagnostic.rule {
                result["ruleId"] = serde_json::json!(rule);
            }
            if let Some(help_url) = &diagnostic.help_url {
                result["helpUri"] = serde_json::json!(help_url);
            }
            result
        })
        .collect::<Vec<_>>();
    let sarif = serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{"tool": {"driver": {"name": "hk"}}, "results": results}],
    });
    xx::file::write(path, &serde_json::to_vec_pretty(&sarif)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcc_supports_windows_paths_multiline_and_duplicates() {
        let output = "C:\\src\\main.c:4:2: warning: first line [W1]\n  continuation\nC:\\src\\main.c:4:2: warning: first line [W1]\n  continuation";
        let parsed = parse(DiagnosticFormat::Gcc, "gcc", "gcc", output);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(
            parsed.diagnostics[0].path.as_deref(),
            Some("C:\\src\\main.c")
        );
        assert!(parsed.diagnostics[0].message.contains("continuation"));
    }

    // The fixtures below are real tool output (paths shortened where noted).
    fn gcc(output: &str) -> ParseResult {
        parse(DiagnosticFormat::Gcc, "step", "tool", output)
    }

    fn at(diagnostic: &Diagnostic) -> (&str, u64, u64) {
        let start = &diagnostic.range.as_ref().unwrap().start;
        (
            diagnostic.path.as_deref().unwrap(),
            start.line,
            start.column,
        )
    }

    #[test]
    fn gcc_ignores_a_tool_prefix_and_package_headers() {
        // go vet 1.25 on a package that does not compile.
        let parsed = gcc(
            "# example.com/m\n# [example.com/m]\nvet: ./main.go:7:14: invalid operation: \"a\" + 1 (mismatched types untyped string and untyped int)\n",
        );
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(at(&parsed.diagnostics[0]), ("./main.go", 7, 14));
        assert_eq!(
            parsed.diagnostics[0].message,
            "invalid operation: \"a\" + 1 (mismatched types untyped string and untyped int)"
        );
        // go vet analyzers, same headers.
        let parsed = gcc(
            "# example.com/m\n# [example.com/m]\n./main.go:9:14: fmt.Printf format %d has arg \"x\" of wrong type string\n",
        );
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(at(&parsed.diagnostics[0]), ("./main.go", 9, 14));
    }

    #[test]
    fn gcc_keeps_a_path_that_starts_with_a_word_and_colon() {
        let parsed = gcc("notes: draft/main.c:4:2: error: bad\n");
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(at(&parsed.diagnostics[0]), ("notes: draft/main.c", 4, 2));
    }

    #[test]
    fn gcc_keeps_preprocessor_source_lines_in_the_message() {
        // gcc 13 with -fno-diagnostics-show-line-numbers
        let output = "inc.c:1:10: fatal error: nonexistent.h: No such file or directory\n #include <nonexistent.h>\n          ^~~~~~~~~~~~~~~\ncompilation terminated.\n";
        let parsed = gcc(output);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert!(
            parsed.diagnostics[0]
                .message
                .contains("#include <nonexistent.h>")
        );
        // clang-format prints source lines unindented, so `#define` and
        // `#include` start the line.
        let output = "fmt.c:1:8: error: code should be clang-formatted [-Wclang-format-violations]\n#define  FOO   1\n       ^\nfmt.c:2:9: error: code should be clang-formatted [-Wclang-format-violations]\n#include  <stdio.h>\n        ^\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(
            parsed.diagnostics[0].message,
            "code should be clang-formatted\n#define  FOO   1\n       ^"
        );
        assert_eq!(
            parsed.diagnostics[1].message,
            "code should be clang-formatted\n#include  <stdio.h>\n        ^"
        );
    }

    #[test]
    fn gcc_does_not_read_source_lines_as_column_less_findings() {
        // clang-format 23 on `//note:12: text` and `//http://x:80: y`
        let output = "cm.c:1:3: error: code should be clang-formatted [-Wclang-format-violations]\n//note:12: text\n  ^\ncm.c:3:3: error: code should be clang-formatted [-Wclang-format-violations]\n//http://x:80: y\n  ^\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(
            parsed.diagnostics[0].message,
            "code should be clang-formatted\n//note:12: text\n  ^"
        );
        assert_eq!(
            parsed.diagnostics[1].message,
            "code should be clang-formatted\n//http://x:80: y\n  ^"
        );
        // Hand-written: source text with a space in the "path", or a bare URL.
        let output = "a.c:1:3: error: bad\n// note:12: text\nhttp://x:80: y\n  ^\n";
        let parsed = gcc(output);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(
            parsed.diagnostics[0].message,
            "bad\n// note:12: text\nhttp://x:80: y\n  ^"
        );
    }

    #[test]
    fn gcc_stops_golangci_lint_findings_at_the_summary() {
        let output = "main.go:10:11: Error return value of `os.Remove` is not checked (errcheck)\n\tos.Remove(\"x\")\n\t         ^\nmain.go:9:14: printf: fmt.Printf format %d has arg \"x\" of wrong type string (govet)\n\tfmt.Printf(\"%d\\n\", \"x\")\n\t            ^\n2 issues:\n* errcheck: 1\n* govet: 1\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(at(&parsed.diagnostics[0]), ("main.go", 10, 11));
        assert_eq!(
            parsed.diagnostics[1].message,
            "printf: fmt.Printf format %d has arg \"x\" of wrong type string (govet)\n\tfmt.Printf(\"%d\\n\", \"x\")\n\t            ^"
        );
        // golangci-lint on a package that does not compile: the first line is
        // a header with a bogus location, then ordinary findings.
        let output = "main.go:1: : # example.com/m\n./main.go:6:2: declared and not used: x\n./main.go:7:14: invalid operation: \"a\" + 1 (mismatched types untyped string and untyped int) (typecheck)\npackage main\n1 issues:\n* typecheck: 1\n";
        let parsed = gcc(output);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(at(&parsed.diagnostics[0]), ("./main.go", 6, 2));
        assert_eq!(parsed.diagnostics[0].message, "declared and not used: x");
        assert!(
            parsed.diagnostics[1]
                .message
                .ends_with("(typecheck)\npackage main")
        );
    }

    #[test]
    fn gcc_reads_mypy_without_a_column_and_ignores_its_summary() {
        let output = "m3.py:2: error: Name \"undefined\" is not defined  [name-defined]\nm2.py:1: error: Incompatible types in assignment (expression has type \"str\", variable has type \"int\")  [assignment]\nm2.py:4: note: Revealed type is \"builtins.int\"\nFound 3 errors in 2 files (checked 2 source files)\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 3);
        let first = &parsed.diagnostics[0];
        assert_eq!(at(first), ("m3.py", 2, 1));
        assert_eq!(first.severity, Severity::Error);
        assert_eq!(first.message, "Name \"undefined\" is not defined");
        assert_eq!(first.rule.as_deref(), Some("name-defined"));
        assert_eq!(parsed.diagnostics[1].rule.as_deref(), Some("assignment"));
        assert_eq!(parsed.diagnostics[2].severity, Severity::Note);
        assert_eq!(parsed.diagnostics[2].rule, None);
        assert_eq!(
            parsed.diagnostics[2].message,
            "Revealed type is \"builtins.int\""
        );
    }

    #[test]
    fn gcc_stops_at_a_blank_line_before_a_summary() {
        // mado
        let output = "t2.md:1:1: MD022 Headers should be surrounded by blank lines\ntest.md:1:1: MD047 File should end with a single newline character\n\nFound 5 errors.\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(
            parsed.diagnostics[1].message,
            "MD047 File should end with a single newline character"
        );
        // rumdl: `[*]` marks an auto-fixable finding and is not a rule.
        let output = "file.md:3:1: [MD012] Multiple consecutive blank lines between content [*]\ntest.md:1:1: [MD041] First line in file should be a level 1 heading\n\nIssues: Found 2 issues in 2 files (13ms)\nRun `rumdl fmt` to automatically fix 1 of the 2 issues\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(
            parsed.diagnostics[0].message,
            "[MD012] Multiple consecutive blank lines between content"
        );
        assert_eq!(parsed.diagnostics[0].rule, None);
        assert_eq!(
            parsed.diagnostics[1].message,
            "[MD041] First line in file should be a level 1 heading"
        );
    }

    #[test]
    fn gcc_ignores_ktlint_log_lines_and_summary() {
        // Paths shortened from the absolute ones ktlint prints.
        let output = "/w/Test.kt:1:17: Unnecessary long whitespace (standard:no-multi-spaces)\n/w/Test.kt:1:26: File must end with a newline (\\n) (standard:final-newline)\n16:01:03.492 [main] WARN com.pinterest.ktlint.cli.internal.KtlintCommandLine -- Lint has found errors than can be autocorrected using 'ktlint --format'\n\nSummary error count (descending) by rule:\n  standard:no-multi-spaces: 2\n  standard:final-newline: 1\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(at(&parsed.diagnostics[0]), ("/w/Test.kt", 1, 17));
        assert_eq!(
            parsed.diagnostics[1].message,
            "File must end with a newline (\\n) (standard:final-newline)"
        );
    }

    #[test]
    fn gcc_reads_buildifier_findings_and_skips_reformat_lines() {
        let output = "B2.bzl:1: module-docstring: The file has no module docstring.\nA module docstring is a string literal (not a comment) which should be the first statement of a file (it may follow comment lines). (https://github.com/bazelbuild/buildtools/blob/main/WARNINGS.md#module-docstring)\nB2.bzl:1: native-cc-library: Function \"cc_library\" is not global anymore and needs to be loaded from \"@rules_cc//cc:cc_library.bzl\". (https://github.com/bazelbuild/buildtools/blob/main/WARNINGS.md#native-cc-library)\nB2.bzl # reformat\nBUILD.bazel:1: native-cc-library: Function \"cc_library\" is not global anymore and needs to be loaded from \"@rules_cc//cc:cc_library.bzl\". (https://github.com/bazelbuild/buildtools/blob/main/WARNINGS.md#native-cc-library)\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 3);
        assert!(parsed.diagnostics[0].message.starts_with(
            "module-docstring: The file has no module docstring.\nA module docstring"
        ));
        assert!(!parsed.diagnostics[1].message.contains("reformat"));
        assert_eq!(at(&parsed.diagnostics[2]), ("BUILD.bazel", 1, 1));
    }

    #[test]
    fn gcc_reads_cpplint_category_and_clamps_line_zero() {
        let output = "main.cc:0:  No copyright message found.  You should have a line: \"Copyright [year] <Copyright Owner>\"  [legal/copyright] [5]\nmain.cc:1:  Missing spaces around =  [whitespace/operators] [4]\nDone processing main.cc\nTotal errors found: 2\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(at(&parsed.diagnostics[0]), ("main.cc", 1, 1));
        assert_eq!(
            parsed.diagnostics[0].rule.as_deref(),
            Some("legal/copyright")
        );
        assert_eq!(parsed.diagnostics[1].message, "Missing spaces around =");
        assert_eq!(
            parsed.diagnostics[1].rule.as_deref(),
            Some("whitespace/operators")
        );
    }

    #[test]
    fn gcc_reads_xmllint_and_ends_blocks_at_blank_source_lines() {
        let output = "bad.xml:3: parser error : Opening and ending tag mismatch: b line 2 and a\n</a>\n    ^\nbad.xml:4: parser error : Premature end of data in tag a line 1\n\n^\nbad2.xml:2: parser error : Opening and ending tag mismatch: b line 2 and c\n  <b></c>\n         ^\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 3);
        assert_eq!(at(&parsed.diagnostics[1]), ("bad.xml", 4, 1));
        assert_eq!(
            parsed.diagnostics[1].message,
            "parser error : Premature end of data in tag a line 1"
        );
        assert_eq!(parsed.diagnostics[2].message.lines().count(), 3);
    }

    #[test]
    fn gcc_keeps_sorbet_context_and_ignores_indented_locations() {
        let output = "t.rb:7: Method `bar` does not exist on `A` https://srb.help/7003\n     7 |A.new.bar\n              ^^^\n  Got `A` originating from:\n    t.rb:7:\n     7 |A.new.bar\n        ^^^^^\n\nt.rb:8: Method `undefined_thing` does not exist on `T.class_of(<root>)` https://srb.help/7003\n     8 |puts undefined_thing\n             ^^^^^^^^^^^^^^^\n  Did you mean `undef_method`? Use `-a` to autocorrect\n    t.rb:8: Replace with `undef_method`\n     8 |puts undefined_thing\n    https://github.com/sorbet/sorbet/tree/15481bbc70595facb3d143d586936aca328de00a/rbi/core/module.rbi#L1811: Defined here\nErrors: 2\n";
        let parsed = gcc(output);
        assert!(parsed.warnings.is_empty());
        assert_eq!(parsed.diagnostics.len(), 2);
        assert_eq!(at(&parsed.diagnostics[0]), ("t.rb", 7, 1));
        assert!(parsed.diagnostics[0].message.contains("t.rb:7:\n"));
        assert_eq!(at(&parsed.diagnostics[1]), ("t.rb", 8, 1));
        assert!(parsed.diagnostics[1].message.ends_with("Defined here"));
    }

    #[test]
    fn gcc_still_warns_about_unrecognized_leading_output() {
        let parsed = gcc("Traceback follows\na.c:1:2: error: bad\n");
        assert_eq!(parsed.warnings.len(), 1);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(parsed.diagnostics[0].message, "bad");
    }

    #[test]
    fn malformed_json_is_a_warning_not_a_panic() {
        let parsed = parse(DiagnosticFormat::EslintJson, "eslint", "eslint", "{");
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.warnings.len(), 1);
    }

    #[test]
    fn cargo_json_normalizes_primary_span_and_rule() {
        let output = r#"{"reason":"compiler-message","message":{"level":"error","message":"bad type","code":{"code":"E1"},"spans":[{"file_name":"src/main.rs","line_start":2,"column_start":3,"line_end":2,"column_end":5,"is_primary":true}]}}"#;
        let parsed = parse(DiagnosticFormat::CargoJson, "cargo", "rustc", output);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(parsed.diagnostics[0].rule.as_deref(), Some("E1"));
        assert_eq!(parsed.diagnostics[0].range.as_ref().unwrap().start.line, 2);
    }

    #[test]
    fn eslint_json_preserves_missing_locations_and_fixes() {
        let output = r#"[{"filePath":"a.js","messages":[{"severity":1,"message":"rename","ruleId":"names","fix":{"text":"ok"}},{"severity":2,"message":"located","line":3,"column":4}]}]"#;
        let parsed = parse(DiagnosticFormat::EslintJson, "eslint", "eslint", output);
        assert_eq!(parsed.diagnostics.len(), 2);
        assert!(parsed.diagnostics[0].range.is_none());
        assert_eq!(
            parsed.diagnostics[0].fix.as_ref().unwrap().replacement,
            "ok"
        );
    }

    #[test]
    fn sarif_normalizes_locations_and_help_urls() {
        let output = r#"{"version":"2.1.0","runs":[{"results":[{"ruleId":"R1","level":"warning","message":{"text":"problem"},"helpUri":"https://example.test/R1","locations":[{"physicalLocation":{"artifactLocation":{"uri":"src/a.rs"},"region":{"startLine":7}}}]}]}]}"#;
        let parsed = parse(DiagnosticFormat::Sarif, "scan", "scanner", output);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(
            parsed.diagnostics[0].help_url.as_deref(),
            Some("https://example.test/R1")
        );
    }

    #[test]
    fn sarif_writer_preserves_help_urls() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("diagnostics.sarif");
        write_sarif(
            &path,
            &[
                Diagnostic {
                    step: "scan".into(),
                    tool: "scanner".into(),
                    severity: Severity::Warning,
                    message: "problem".into(),
                    path: None,
                    range: None,
                    rule: Some("R1".into()),
                    help_url: Some("https://example.test/R1".into()),
                    fix: None,
                },
                Diagnostic {
                    step: "scan".into(),
                    tool: "scanner".into(),
                    severity: Severity::Note,
                    message: "location only".into(),
                    path: Some("src/main.rs".into()),
                    range: Some(Range {
                        start: Position { line: 2, column: 3 },
                        end: None,
                    }),
                    rule: None,
                    help_url: None,
                    fix: None,
                },
            ],
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(
            value
                .pointer("/runs/0/results/0/helpUri")
                .and_then(Value::as_str),
            Some("https://example.test/R1")
        );
        let location_only = &value["runs"][0]["results"][1];
        assert!(location_only.get("ruleId").is_none());
        assert!(location_only.get("helpUri").is_none());
        let region = &location_only["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 2);
        assert!(region.get("endLine").is_none());
        assert!(region.get("endColumn").is_none());
    }

    fn diag(path: Option<&str>) -> Diagnostic {
        Diagnostic {
            step: "s".into(),
            tool: "t".into(),
            severity: Severity::Error,
            message: "m".into(),
            path: path.map(str::to_string),
            range: None,
            rule: None,
            help_url: None,
            fix: None,
        }
    }

    fn rebased(dir: &str, path: &str) -> String {
        let mut d = [diag(Some(path))];
        rebase_paths(&mut d, dir);
        d[0].path.clone().unwrap()
    }

    #[test]
    fn rebase_prefixes_relative_paths_with_the_dir() {
        assert_eq!(rebased("svc/api", "main.go"), "svc/api/main.go");
        assert_eq!(rebased("svc/api", "./pkg/a.go"), "svc/api/pkg/a.go");
        assert_eq!(rebased("./svc/api/", "main.go"), "svc/api/main.go");
    }

    #[test]
    fn rebase_resolves_parent_segments() {
        assert_eq!(rebased("svc/api", "../shared/x.go"), "svc/shared/x.go");
        assert_eq!(rebased("svc/api", "../../x.go"), "x.go");
        assert_eq!(rebased("svc", "../../x.go"), "../x.go");
    }

    #[test]
    fn rebase_handles_windows_separators() {
        assert_eq!(rebased("svc\\api", "pkg\\a.go"), "svc/api/pkg/a.go");
        assert_eq!(rebased("svc", ".\\a.go"), "svc/a.go");
    }

    #[test]
    fn rebase_leaves_absolute_paths_and_uris_alone() {
        assert_eq!(rebased("svc", "/abs/a.go"), "/abs/a.go");
        assert_eq!(rebased("svc", "C:\\src\\a.go"), "C:\\src\\a.go");
        assert_eq!(rebased("svc", "C:/src/a.go"), "C:/src/a.go");
        assert_eq!(rebased("svc", "file:///abs/a.go"), "file:///abs/a.go");
        assert_eq!(rebased("svc", "a:b.go"), "svc/a:b.go");
    }

    #[test]
    fn rebase_is_a_no_op_for_the_repo_root_and_absolute_dirs() {
        assert_eq!(rebased(".", "a.go"), "a.go");
        assert_eq!(rebased("", "./a.go"), "./a.go");
        assert_eq!(rebased("/abs/dir", "a.go"), "a.go");
    }

    #[test]
    fn rebase_covers_missing_paths_and_fix_paths() {
        let mut d = [diag(None)];
        d[0].fix = Some(DiagnosticFix {
            replacement: "x".into(),
            path: Some("a.go".into()),
            range: None,
        });
        rebase_paths(&mut d, "svc");
        assert_eq!(d[0].path, None);
        assert_eq!(d[0].fix.as_ref().unwrap().path.as_deref(), Some("svc/a.go"));
    }

    #[test]
    fn segments_are_rebased_per_directory() {
        let parsed = parse_segments(
            DiagnosticFormat::Gcc,
            "s",
            "t",
            [
                (Some("a"), "x.go:1:2: error: bad"),
                (Some("b"), "x.go:1:2: error: bad"),
                (None, "y.go:3:4: warning: meh"),
            ],
        );
        let paths: Vec<_> = parsed
            .diagnostics
            .iter()
            .map(|d| d.path.as_deref().unwrap())
            .collect();
        assert_eq!(paths, ["a/x.go", "b/x.go", "y.go"]);
    }

    #[test]
    fn json_segments_are_parsed_separately_not_concatenated() {
        let eslint = |file: &str| {
            format!(
                r#"[{{"filePath":"{file}","messages":[{{"severity":2,"message":"bad","line":1,"column":1}}]}}]"#
            )
        };
        let sarif = |file: &str| {
            format!(
                r#"{{"version":"2.1.0","runs":[{{"results":[{{"ruleId":"R1","level":"warning","message":{{"text":"p"}},"locations":[{{"physicalLocation":{{"artifactLocation":{{"uri":"{file}"}},"region":{{"startLine":1}}}}}}]}}]}}]}}"#
            )
        };
        for (format, make) in [
            (
                DiagnosticFormat::EslintJson,
                &eslint as &dyn Fn(&str) -> String,
            ),
            (DiagnosticFormat::Sarif, &sarif),
        ] {
            let (a, b) = (make("x.js"), make("y.js"));
            let parsed = parse_segments(
                format,
                "s",
                "t",
                [(Some("pkg/a"), a.as_str()), (Some("pkg/b"), b.as_str())],
            );
            assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
            let paths: Vec<_> = parsed
                .diagnostics
                .iter()
                .map(|d| d.path.as_deref().unwrap())
                .collect();
            assert_eq!(paths, ["pkg/a/x.js", "pkg/b/y.js"]);
        }
    }
}
