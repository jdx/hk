//! CI-native renderings of a run result: GitHub Actions workflow-command
//! annotations and job summary, and GitLab Code Quality reports. Unlike SARIF
//! upload, none of these need GitHub Advanced Security, so they work on
//! private repositories too.

use super::{RunResult, StepResult};
use crate::diagnostics::{Diagnostic, Severity};
use std::fmt::Write as _;

/// Most bytes of a failed step's output shown in one annotation. GitHub shows
/// annotations in a narrow panel; the full output stays in the job log.
const MAX_ANNOTATION_OUTPUT_BYTES: usize = 4 * 1024;
/// Most findings listed in the job summary table. GitHub caps a step summary
/// at 1 MiB; findings past this are counted, not listed.
const MAX_SUMMARY_FINDINGS: usize = 500;
/// Most bytes of a failed step's output embedded in the job summary.
const MAX_SUMMARY_OUTPUT_BYTES: usize = 16 * 1024;

/// A step that failed the run (allowed failures do not).
fn failed(step: &StepResult) -> bool {
    step.status == "failed" && !step.failure_allowed
}

/// Escape a workflow command's message.
fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape a workflow command property value.
fn escape_property(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

fn truncate(s: &str, max: usize) -> (&str, bool) {
    if s.len() <= max {
        return (s, false);
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    (&s[..end], true)
}

fn diagnostic_title(diagnostic: &Diagnostic) -> String {
    match &diagnostic.rule {
        Some(rule) => format!("{} ({rule})", diagnostic.tool),
        None => diagnostic.tool.clone(),
    }
}

/// Render GitHub Actions workflow commands (`::error file=…::message`) for
/// every diagnostic, for each failed step that produced no diagnostics, and
/// for a run that failed outside any step.
pub(super) fn github_annotations(run: &RunResult) -> String {
    let mut out = String::new();
    for step in &run.steps {
        for diagnostic in &step.diagnostics {
            let command = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Note | Severity::Help => "notice",
            };
            let mut props = Vec::new();
            if let Some(path) = &diagnostic.path {
                props.push(format!("file={}", escape_property(path)));
                if let Some(range) = &diagnostic.range {
                    props.push(format!("line={}", range.start.line));
                    if range.start.column > 0 {
                        props.push(format!("col={}", range.start.column));
                    }
                    if let Some(end) = &range.end {
                        props.push(format!("endLine={}", end.line));
                        // GitHub ignores columns on multi-line annotations.
                        if end.line == range.start.line && end.column > 0 {
                            props.push(format!("endColumn={}", end.column));
                        }
                    }
                }
            }
            props.push(format!(
                "title={}",
                escape_property(&diagnostic_title(diagnostic))
            ));
            let mut message = diagnostic.message.clone();
            if let Some(url) = &diagnostic.help_url {
                write!(message, "\n{url}").unwrap();
            }
            writeln!(
                out,
                "::{command} {}::{}",
                props.join(","),
                escape_data(&message)
            )
            .unwrap();
        }
        if failed(step) && step.diagnostics.is_empty() {
            let mut message = format!("hk step `{}` failed", step.name);
            if let Some(output) = step.output.as_deref().map(str::trim)
                && !output.is_empty()
            {
                let (output, cut) = truncate(output, MAX_ANNOTATION_OUTPUT_BYTES);
                write!(message, "\n\n{output}").unwrap();
                if cut {
                    message.push_str("\n[truncated; see the job log]");
                }
            }
            writeln!(
                out,
                "::error title={}::{}",
                escape_property(&format!("hk: {}", step.name)),
                escape_data(&message)
            )
            .unwrap();
        }
    }
    if let Some(failure) = &run.failure
        && !run.steps.iter().any(failed)
    {
        writeln!(
            out,
            "::error title={}::{}",
            escape_property(&format!("hk {}", run.hook)),
            escape_data(failure)
        )
        .unwrap();
    }
    out
}

/// Escape text for a Markdown table cell.
fn md_cell(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '|' => out.push_str("\\|"),
            '`' => out.push_str("&#96;"),
            '\r' => {}
            '\n' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// A Markdown code fence longer than any backtick run in `s`.
fn fence(s: &str) -> String {
    let mut longest = 0;
    let mut current = 0;
    for c in s.chars() {
        if c == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    "`".repeat((longest + 1).max(3))
}

fn status_label(step: &StepResult) -> &'static str {
    match step.status {
        "failed" if step.failure_allowed => "⚠️ failed (allowed)",
        "failed" => "❌ failed",
        "passed" => "✅ passed",
        "skipped" => "⏭️ skipped",
        "cancelled" => "🚫 cancelled",
        _ => "",
    }
}

fn location(diagnostic: &Diagnostic) -> String {
    let Some(path) = &diagnostic.path else {
        return String::new();
    };
    match &diagnostic.range {
        Some(range) if range.start.column > 0 => {
            format!("{path}:{}:{}", range.start.line, range.start.column)
        }
        Some(range) => format!("{path}:{}", range.start.line),
        None => path.clone(),
    }
}

/// Render a Markdown job summary: the run status, a table of steps, every
/// finding, and the output of failed steps that produced no findings.
pub(super) fn github_summary(run: &RunResult) -> String {
    let mut out = String::new();
    let icon = match run.status {
        "passed" => "✅",
        "cancelled" => "🚫",
        _ => "❌",
    };
    writeln!(
        out,
        "## {icon} hk {}: {} ({:.1}s)\n",
        md_cell(&run.hook),
        run.status,
        run.duration_ms as f64 / 1000.0
    )
    .unwrap();
    if let Some(failure) = &run.failure {
        let fence = fence(failure);
        writeln!(out, "{fence}\n{}\n{fence}\n", failure.trim_end()).unwrap();
    }
    if let Some(reason) = &run.reason {
        writeln!(out, "{}\n", md_cell(reason)).unwrap();
    }
    let findings: Vec<&Diagnostic> = run
        .steps
        .iter()
        .flat_map(|step| step.diagnostics.iter())
        .collect();
    let ran: Vec<&StepResult> = run
        .steps
        .iter()
        .filter(|step| step.status != "skipped")
        .collect();
    if !ran.is_empty() {
        out.push_str("| Step | Status | Duration | Findings |\n| --- | --- | ---: | ---: |\n");
        for step in &ran {
            writeln!(
                out,
                "| {} | {} | {:.1}s | {} |",
                md_cell(&step.name),
                status_label(step),
                step.duration_ms as f64 / 1000.0,
                step.diagnostics.len()
            )
            .unwrap();
        }
        out.push('\n');
    }
    let skipped = run.steps.len() - ran.len();
    if skipped > 0 {
        writeln!(
            out,
            "{skipped} step{} skipped.\n",
            if skipped == 1 { "" } else { "s" }
        )
        .unwrap();
    }
    if !findings.is_empty() {
        writeln!(out, "### Findings ({})\n", findings.len()).unwrap();
        out.push_str("| Location | Severity | Tool | Message |\n| --- | --- | --- | --- |\n");
        for diagnostic in findings.iter().take(MAX_SUMMARY_FINDINGS) {
            let location = location(diagnostic);
            let tool = diagnostic_title(diagnostic);
            let tool = match &diagnostic.help_url {
                Some(url) => format!("[{}]({})", md_cell(&tool), url.replace(')', "%29")),
                None => md_cell(&tool),
            };
            writeln!(
                out,
                "| {} | {} | {} | {} |",
                if location.is_empty() {
                    String::new()
                } else {
                    format!("<code>{}</code>", md_cell(&location))
                },
                match diagnostic.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Note => "note",
                    Severity::Help => "help",
                },
                tool,
                md_cell(&diagnostic.message)
            )
            .unwrap();
        }
        if findings.len() > MAX_SUMMARY_FINDINGS {
            writeln!(
                out,
                "\n{} more findings not shown; use `--sarif` or `--format json` for all of them.",
                findings.len() - MAX_SUMMARY_FINDINGS
            )
            .unwrap();
        }
        out.push('\n');
    }
    for step in run.steps.iter().filter(|step| failed(step)) {
        if !step.diagnostics.is_empty() {
            continue;
        }
        let Some(output) = step.output.as_deref().map(str::trim_end) else {
            continue;
        };
        if output.trim().is_empty() {
            continue;
        }
        let (output, cut) = truncate(output, MAX_SUMMARY_OUTPUT_BYTES);
        let fence = fence(output);
        writeln!(
            out,
            "<details><summary>Output of <code>{}</code></summary>\n\n{fence}\n{output}\n{fence}\n{}</details>\n",
            md_cell(&step.name),
            if cut { "\nTruncated; see the job log.\n" } else { "" }
        )
        .unwrap();
    }
    out
}

/// Render a GitLab Code Quality report (a subset of the Code Climate format).
/// Findings without a file path are omitted because GitLab requires one.
pub(super) fn gitlab_code_quality(run: &RunResult) -> serde_json::Value {
    let mut seen = std::collections::HashMap::<String, usize>::new();
    let issues = run
        .steps
        .iter()
        .flat_map(|step| step.diagnostics.iter())
        .filter_map(|diagnostic| {
            let path = diagnostic.path.as_ref()?;
            let check_name = match &diagnostic.rule {
                Some(rule) => format!("{}/{rule}", diagnostic.tool),
                None => diagnostic.tool.clone(),
            };
            // GitLab matches fingerprints between the base and head reports
            // to tell new findings from fixed ones, so leave the line number
            // out (moving code should not create a "new" finding) and number
            // repeats of the same finding in one file instead.
            let key = crate::hash::hash_to_str(&(
                &diagnostic.step,
                &check_name,
                path,
                &diagnostic.message,
            ));
            let occurrence = seen.entry(key.clone()).or_default();
            *occurrence += 1;
            let fingerprint = if *occurrence == 1 {
                key
            } else {
                crate::hash::hash_to_str(&(key, *occurrence))
            };
            let line = diagnostic
                .range
                .as_ref()
                .map(|range| range.start.line.max(1))
                .unwrap_or(1);
            Some(serde_json::json!({
                "type": "issue",
                "description": diagnostic.message,
                "check_name": check_name,
                "fingerprint": fingerprint,
                "severity": match diagnostic.severity {
                    Severity::Error => "major",
                    Severity::Warning => "minor",
                    Severity::Note | Severity::Help => "info",
                },
                "location": {"path": path, "lines": {"begin": line}},
            }))
        })
        .collect();
    serde_json::Value::Array(issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{Position, Range};

    fn diagnostic(severity: Severity, path: Option<&str>, line: u64) -> Diagnostic {
        Diagnostic {
            step: "lint".into(),
            tool: "eslint".into(),
            severity,
            message: "bad: thing, 100%\nsecond".into(),
            path: path.map(Into::into),
            range: path.map(|_| Range {
                start: Position { line, column: 3 },
                end: None,
            }),
            rule: Some("no-x".into()),
            help_url: None,
            fix: None,
        }
    }

    fn step(name: &str, status: &'static str, diagnostics: Vec<Diagnostic>) -> StepResult {
        StepResult {
            name: name.into(),
            status,
            failure_allowed: false,
            duration_ms: 1200,
            effects: vec![],
            diagnostics,
            parse_warnings: vec![],
            output_kind: None,
            output: Some("boom | <b>\n".into()),
            skip_reason: None,
        }
    }

    fn run(steps: Vec<StepResult>, failure: Option<&str>) -> RunResult {
        RunResult {
            schema_version: 1,
            kind: "run_result",
            hook: "check".into(),
            status: if failure.is_some() {
                "failed"
            } else {
                "passed"
            },
            started_at: String::new(),
            duration_ms: 1500,
            failure: failure.map(Into::into),
            reason: None,
            steps,
        }
    }

    #[test]
    fn annotations_escape_messages_and_properties() {
        let run = run(
            vec![step(
                "lint",
                "failed",
                vec![diagnostic(Severity::Warning, Some("a,b:c.js"), 4)],
            )],
            Some("1 step failed"),
        );
        assert_eq!(
            github_annotations(&run),
            "::warning file=a%2Cb%3Ac.js,line=4,col=3,title=eslint (no-x)::bad: thing, 100%25%0Asecond\n"
        );
    }

    #[test]
    fn annotations_cover_failed_steps_without_diagnostics() {
        let run = run(
            vec![step("fmt", "failed", vec![]), step("ok", "passed", vec![])],
            Some("1 step failed"),
        );
        assert_eq!(
            github_annotations(&run),
            "::error title=hk%3A fmt::hk step `fmt` failed%0A%0Aboom | <b>\n"
        );
    }

    #[test]
    fn annotations_report_setup_failures() {
        let run = run(vec![], Some("git status failed"));
        assert_eq!(
            github_annotations(&run),
            "::error title=hk check::git status failed\n"
        );
    }

    #[test]
    fn allowed_failures_are_not_annotated() {
        let mut allowed = step("fmt", "failed", vec![]);
        allowed.failure_allowed = true;
        assert_eq!(github_annotations(&run(vec![allowed], None)), "");
    }

    #[test]
    fn summary_lists_steps_findings_and_output() {
        let run = run(
            vec![
                step(
                    "lint",
                    "failed",
                    vec![diagnostic(Severity::Error, Some("a.js"), 4)],
                ),
                step("fmt", "failed", vec![]),
            ],
            Some("2 steps failed"),
        );
        let summary = github_summary(&run);
        assert!(summary.starts_with("## ❌ hk check: failed (1.5s)\n"));
        assert!(summary.contains("| lint | ❌ failed | 1.2s | 1 |"));
        assert!(summary.contains(
            "| <code>a.js:4:3</code> | error | eslint (no-x) | bad: thing, 100% second |"
        ));
        assert!(summary.contains("Output of <code>fmt</code>"));
        assert!(summary.contains("```\nboom | <b>\n```"));
    }

    #[test]
    fn code_quality_requires_paths_and_has_unique_fingerprints() {
        let run = run(
            vec![step(
                "lint",
                "failed",
                vec![
                    diagnostic(Severity::Error, Some("a.js"), 4),
                    diagnostic(Severity::Error, Some("a.js"), 9),
                    diagnostic(Severity::Note, None, 0),
                ],
            )],
            None,
        );
        let report = gitlab_code_quality(&run);
        let issues = report.as_array().unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0]["check_name"], "eslint/no-x");
        assert_eq!(issues[0]["severity"], "major");
        assert_eq!(issues[0]["location"]["path"], "a.js");
        assert_eq!(issues[0]["location"]["lines"]["begin"], 4);
        assert_ne!(issues[0]["fingerprint"], issues[1]["fingerprint"]);
    }
}
