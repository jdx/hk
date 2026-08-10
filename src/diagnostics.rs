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

fn parse_gcc(step: &str, tool: &str, output: &str) -> ParseResult {
    let regex = regex::Regex::new(
        r"^(.*?):(\d+):(\d+):\s*(?:(error|warning|note|help):\s*)?(.*?)(?:\s+\[([^\]]+)\])?$",
    )
    .expect("valid GCC diagnostic regex");
    let mut parsed = ParseResult::default();
    for line in output.lines() {
        if let Some(captures) = regex.captures(line) {
            parsed.diagnostics.push(Diagnostic {
                step: step.to_string(),
                tool: tool.to_string(),
                severity: severity(captures.get(4).map_or("error", |value| value.as_str())),
                message: captures[5].to_string(),
                path: Some(captures[1].to_string()),
                range: Some(Range {
                    start: Position {
                        line: captures[2].parse().unwrap_or(1),
                        column: captures[3].parse().unwrap_or(1),
                    },
                    end: None,
                }),
                rule: captures.get(6).map(|value| value.as_str().to_string()),
                help_url: None,
                fix: None,
            });
        } else if !line.trim().is_empty() {
            if let Some(previous) = parsed.diagnostics.last_mut() {
                previous.message.push('\n');
                previous.message.push_str(line);
            } else {
                parsed
                    .warnings
                    .push(format!("unrecognized GCC diagnostic: {line}"));
            }
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
                serde_json::json!({
                    "physicalLocation": {
                        "artifactLocation": {"uri": path},
                        "region": diagnostic.range.as_ref().map(|range| serde_json::json!({
                            "startLine": range.start.line,
                            "startColumn": range.start.column,
                            "endLine": range.end.as_ref().map(|end| end.line),
                            "endColumn": range.end.as_ref().map(|end| end.column),
                        }))
                    }
                })
            });
            serde_json::json!({
                "ruleId": diagnostic.rule,
                "level": match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Note | Severity::Help => "note",
                },
                "message": {"text": diagnostic.message},
                "helpUri": diagnostic.help_url,
                "locations": location.into_iter().collect::<Vec<_>>(),
                "properties": {"step": diagnostic.step, "tool": diagnostic.tool},
            })
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
            &[Diagnostic {
                step: "scan".into(),
                tool: "scanner".into(),
                severity: Severity::Warning,
                message: "problem".into(),
                path: None,
                range: None,
                rule: Some("R1".into()),
                help_url: Some("https://example.test/R1".into()),
                fix: None,
            }],
        )
        .unwrap();
        let value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(
            value
                .pointer("/runs/0/results/0/helpUri")
                .and_then(Value::as_str),
            Some("https://example.test/R1")
        );
    }
}
