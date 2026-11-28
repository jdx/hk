//! CI-native renderings of a run result: GitHub Actions workflow-command
//! annotations and job summary, and GitLab Code Quality reports. Unlike SARIF
//! upload, none of these need GitHub Advanced Security, so they work on
//! private repositories too.

use super::{RunResult, StepResult};
use crate::diagnostics::{Diagnostic, Severity};
use std::borrow::Cow;
use std::fmt::Write as _;
use std::path::Path;

/// Most bytes of a failed step's output shown in one annotation. GitHub shows
/// annotations in a narrow panel; the full output stays in the job log.
const MAX_ANNOTATION_OUTPUT_BYTES: usize = 4 * 1024;
/// Most findings listed in the job summary table; findings past this are
/// counted, not listed.
const MAX_SUMMARY_FINDINGS: usize = 500;
/// Most bytes of a failed step's output embedded in the job summary.
const MAX_SUMMARY_OUTPUT_BYTES: usize = 16 * 1024;
/// Most bytes of one finding's message in the job summary table.
const MAX_SUMMARY_MESSAGE_BYTES: usize = 1024;
/// Most bytes of a step's whole job summary file. GitHub drops a step summary
/// larger than 1 MiB, so stay well under it, leaving room for the truncation
/// notice.
pub(super) const MAX_SUMMARY_BYTES: usize = 768 * 1024;

/// A diagnostic path relative to the repository root, which both GitHub and
/// GitLab require. Tools such as ESLint report absolute paths; hk runs from
/// the work tree root, so strip the current directory from paths under it.
fn repo_path(path: &str) -> Cow<'_, str> {
    let as_path = Path::new(path);
    if as_path.is_absolute()
        && let Ok(cwd) = std::env::current_dir()
    {
        let canonical = cwd.canonicalize().ok();
        let relative = as_path.strip_prefix(&cwd).ok().or_else(|| {
            canonical
                .as_ref()
                .and_then(|canonical| as_path.strip_prefix(canonical).ok())
        });
        if let Some(relative) = relative {
            let relative = crate::hook::with_forward_slashes(relative.to_path_buf());
            return Cow::Owned(relative.to_string_lossy().into_owned());
        }
    }
    match path.strip_prefix("./") {
        Some(stripped) => Cow::Borrowed(stripped),
        None => Cow::Borrowed(path),
    }
}

/// A step that failed the run (allowed failures do not).
fn failed(step: &StepResult) -> bool {
    step.status == "failed" && !step.failure_allowed
}

/// A step whose failure was allowed: the run passes despite it, so its
/// findings are not raised as annotations.
fn failure_allowed(step: &StepResult) -> bool {
    step.status == "failed" && step.failure_allowed
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
    for step in run.steps.iter().filter(|step| !failure_allowed(step)) {
        for diagnostic in &step.diagnostics {
            let command = match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Note | Severity::Help => "notice",
            };
            let mut props = Vec::new();
            if let Some(path) = &diagnostic.path {
                props.push(format!("file={}", escape_property(&repo_path(path))));
                // Lines are 1-based; a line 0 marks a file-level finding.
                if let Some(range) = diagnostic.range.as_ref().filter(|r| r.start.line > 0) {
                    props.push(format!("line={}", range.start.line));
                    let end_line = range
                        .end
                        .as_ref()
                        .map(|end| end.line)
                        .filter(|line| *line > range.start.line);
                    match end_line {
                        // GitHub rejects columns on a multi-line annotation.
                        Some(end_line) => props.push(format!("endLine={end_line}")),
                        None => {
                            if range.start.column > 0 {
                                props.push(format!("col={}", range.start.column));
                            }
                            if let Some(end) = range.end.as_ref().filter(|end| {
                                end.line == range.start.line && end.column > range.start.column
                            }) {
                                props.push(format!("endColumn={}", end.column));
                            }
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
    let path = repo_path(path);
    match &diagnostic.range {
        Some(range) if range.start.column > 0 => {
            format!("{path}:{}:{}", range.start.line, range.start.column)
        }
        Some(range) => format!("{path}:{}", range.start.line),
        None => path.into_owned(),
    }
}

/// Render a Markdown job summary: the run status, a table of steps, every
/// finding, and the output of failed steps that produced no findings.
/// `budget` is the most bytes it may take; the caller subtracts whatever the
/// step's summary file already holds.
pub(super) fn github_summary(run: &RunResult, budget: usize) -> String {
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
    let mut truncated = false;
    if !findings.is_empty() {
        writeln!(out, "### Findings ({})\n", findings.len()).unwrap();
        out.push_str("| Location | Severity | Tool | Message |\n| --- | --- | --- | --- |\n");
        let mut listed = 0;
        for diagnostic in findings.iter().take(MAX_SUMMARY_FINDINGS) {
            let mut row = String::new();
            let location = location(diagnostic);
            let tool = diagnostic_title(diagnostic);
            let tool = match &diagnostic.help_url {
                Some(url) => format!("[{}]({})", md_cell(&tool), url.replace(')', "%29")),
                None => md_cell(&tool),
            };
            let (message, cut) = truncate(&diagnostic.message, MAX_SUMMARY_MESSAGE_BYTES);
            writeln!(
                row,
                "| {} | {} | {} | {}{} |",
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
                md_cell(message),
                if cut { " …" } else { "" }
            )
            .unwrap();
            if out.len() + row.len() > budget {
                truncated = true;
                break;
            }
            out.push_str(&row);
            listed += 1;
        }
        if listed < findings.len() {
            writeln!(
                out,
                "\n{} more findings not shown; use `--sarif` or `--format json` for all of them.",
                findings.len() - listed
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
        let mut details = String::new();
        writeln!(
            details,
            "<details><summary>Output of <code>{}</code></summary>\n\n{fence}\n{output}\n{fence}\n{}</details>\n",
            md_cell(&step.name),
            if cut { "\nTruncated; see the job log.\n" } else { "" }
        )
        .unwrap();
        if out.len() + details.len() > budget {
            truncated = true;
            continue;
        }
        out.push_str(&details);
    }
    if truncated {
        out.push_str(
            "Summary truncated to stay under GitHub's size limit; see the job log for the rest.\n",
        );
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
            let path = repo_path(diagnostic.path.as_ref()?);
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
                &*path,
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
    fn annotations_drop_columns_on_multi_line_ranges_and_line_zero() {
        let mut multi = diagnostic(Severity::Error, Some("a.js"), 4);
        multi.range.as_mut().unwrap().end = Some(Position { line: 6, column: 2 });
        let mut single = diagnostic(Severity::Error, Some("b.js"), 4);
        single.range.as_mut().unwrap().end = Some(Position { line: 4, column: 9 });
        let file_level = diagnostic(Severity::Error, Some("c.js"), 0);
        let out = github_annotations(&run(
            vec![step("lint", "failed", vec![multi, single, file_level])],
            None,
        ));
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].starts_with("::error file=a.js,line=4,endLine=6,title="));
        assert!(lines[1].starts_with("::error file=b.js,line=4,col=3,endColumn=9,title="));
        assert!(lines[2].starts_with("::error file=c.js,title="));
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
        let mut with_findings = step(
            "lint",
            "failed",
            vec![diagnostic(Severity::Error, Some("a.js"), 4)],
        );
        with_findings.failure_allowed = true;
        assert_eq!(
            github_annotations(&run(vec![allowed, with_findings], None)),
            ""
        );
    }

    #[test]
    fn absolute_paths_under_the_work_tree_become_relative() {
        let cwd = std::env::current_dir().unwrap();
        let absolute = cwd.join("src").join("a.js");
        let run = run(
            vec![step(
                "lint",
                "failed",
                vec![diagnostic(
                    Severity::Error,
                    Some(absolute.to_str().unwrap()),
                    4,
                )],
            )],
            None,
        );
        assert!(github_annotations(&run).starts_with("::error file=src/a.js,"));
        assert_eq!(gitlab_code_quality(&run)[0]["location"]["path"], "src/a.js");
        assert_eq!(repo_path("/elsewhere/a.js"), "/elsewhere/a.js");
        assert_eq!(repo_path("./a.js"), "a.js");
        if cfg!(unix) {
            // A backslash is a legal filename character on Unix.
            let literal = cwd.join("a\\b.js");
            assert_eq!(repo_path(literal.to_str().unwrap()), "a\\b.js");
        }
    }

    #[test]
    fn summary_stays_under_the_size_budget() {
        let mut long = diagnostic(Severity::Error, Some("a.js"), 4);
        long.message = "x".repeat(64 * 1024);
        let mut steps = vec![step("lint", "failed", vec![long; MAX_SUMMARY_FINDINGS])];
        for i in 0..100 {
            let mut failed = step(&format!("fmt{i}"), "failed", vec![]);
            failed.output = Some("y".repeat(64 * 1024));
            steps.push(failed);
        }
        let summary = github_summary(&run(steps, Some("failed")), MAX_SUMMARY_BYTES);
        assert!(
            summary.len() <= MAX_SUMMARY_BYTES + 1024,
            "{}",
            summary.len()
        );
        assert!(summary.ends_with("see the job log for the rest.\n"));
        assert!(summary.contains("more findings not shown") || summary.contains("x …"));
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
        let summary = github_summary(&run, MAX_SUMMARY_BYTES);
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
