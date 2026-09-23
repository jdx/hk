use crate::{
    Result,
    diagnostics::{self, Diagnostic},
    hook::HookContext,
    step::{CommandEffect, OutputSummary},
};
use serde::Serialize;
use std::path::Path;
use std::{
    io::Write,
    sync::{Mutex, OnceLock},
};

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Eq,
    PartialEq,
    usage_rs::ValueEnum,
    strum::EnumString,
    strum::Display,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Human,
    Json,
    Jsonl,
}

#[derive(Debug, Serialize)]
struct RunResult {
    schema_version: u8,
    kind: &'static str,
    hook: String,
    status: &'static str,
    started_at: String,
    duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    steps: Vec<StepResult>,
}

#[derive(Debug, Clone, Serialize)]
struct StepResult {
    name: String,
    status: &'static str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    failure_allowed: bool,
    duration_ms: u128,
    effects: Vec<ExecutedEffect>,
    diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parse_warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_kind: Option<OutputSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutedEffect {
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    effect: Option<CommandEffect>,
}

#[derive(Debug, Serialize)]
struct Event<'a, T: Serialize> {
    schema_version: u8,
    event: &'a str,
    sequence: usize,
    data: T,
}

fn jsonl_sequence() -> &'static Mutex<usize> {
    static SEQUENCE: OnceLock<Mutex<usize>> = OnceLock::new();
    SEQUENCE.get_or_init(|| Mutex::new(0))
}

pub fn emit_run_started(format: OutputFormat, hook: &str, started_at: &str) -> Result<()> {
    if format != OutputFormat::Jsonl {
        return Ok(());
    }
    let mut sequence = jsonl_sequence().lock().unwrap();
    *sequence = 0;
    write_event(
        &mut std::io::stdout().lock(),
        &Event {
            schema_version: 1,
            event: "run_started",
            sequence: *sequence,
            data: serde_json::json!({
                "hook": hook,
                "started_at": started_at,
            }),
        },
    )?;
    *sequence += 1;
    Ok(())
}

pub fn emit_step_started(format: OutputFormat, name: &str) -> Result<()> {
    if format != OutputFormat::Jsonl {
        return Ok(());
    }
    write_jsonl_event(
        "step_started",
        serde_json::json!({
            "name": name,
            "status": "running",
            "started_at": chrono::Utc::now().to_rfc3339(),
            "duration_ms": 0,
            "effects": [],
            "diagnostics": [],
        }),
    )
}

pub fn emit_run_planned(format: OutputFormat, steps: &[String]) -> Result<()> {
    if format != OutputFormat::Jsonl {
        return Ok(());
    }
    write_jsonl_event(
        "run_planned",
        serde_json::json!({
            "steps": steps.iter().map(|name| serde_json::json!({
                "name": name,
                "status": "pending",
                "duration_ms": 0,
                "effects": [],
                "diagnostics": [],
            })).collect::<Vec<_>>(),
        }),
    )
}

pub fn emit_step_completed(format: OutputFormat, name: &str, status: &str) -> Result<()> {
    if format != OutputFormat::Jsonl {
        return Ok(());
    }
    write_jsonl_event(
        "step_completed",
        serde_json::json!({
            "name": name,
            "status": status,
            "duration_ms": 0,
            "effects": [],
            "diagnostics": [],
        }),
    )
}

/// File-based report formats to write alongside the primary structured
/// output, keyed by CLI flag (`--sarif`, `--junit-xml`).
#[derive(Debug, Clone, Copy, Default)]
pub struct ReportPaths<'a> {
    pub sarif: Option<&'a Path>,
    pub junit: Option<&'a Path>,
}

pub fn emit_run(
    format: OutputFormat,
    hook: &str,
    started_at: String,
    duration_ms: u128,
    ctx: &HookContext,
    failure: Option<String>,
    reports: ReportPaths,
) -> Result<()> {
    let failed = ctx.failed_steps.lock().unwrap();
    let allowed_failures = ctx.allowed_failure_steps.lock().unwrap();
    let finished = ctx.finished_steps.lock().unwrap();
    let cancelled = ctx.cancelled_steps.lock().unwrap();
    let run_was_cancelled = !cancelled.is_empty();
    let skipped = ctx.get_skipped_steps();
    let outputs = ctx.output_by_step.lock().unwrap();
    let diagnostic_outputs = ctx.diagnostic_output_by_step.lock().unwrap();
    let executed_effects = ctx.command_effects_by_step.lock().unwrap();
    let timings = ctx.timing.step_wall_times();
    let mut steps = Vec::new();
    for group in &ctx.groups {
        for name in group.steps.keys() {
            let skip_reason = skipped.get(name).map(|reason| reason.message());
            // A cancelled step can also retain output from an earlier failed
            // check. Cancellation describes the final step outcome and must
            // therefore take precedence over that diagnostic bookkeeping.
            let status = if cancelled.contains(name) {
                "cancelled"
            } else if failed.contains(name) {
                "failed"
            } else if skip_reason.is_some() {
                "skipped"
            } else if finished.contains(name) {
                "passed"
            } else {
                "cancelled"
            };
            let (mut output_kind, mut output) = outputs
                .get(name)
                .map(|(kind, output)| (Some(kind.clone()), Some(output.clone())))
                .unwrap_or((None, None));
            let step = &group.steps[name];
            let diagnostic_output = diagnostic_outputs.get(name);
            if let Some(diagnostic_output) = diagnostic_output {
                output_kind.get_or_insert_with(|| step.output_summary.clone());
                match &mut output {
                    Some(output) if !output.contains(diagnostic_output) => {
                        output.insert_str(0, diagnostic_output)
                    }
                    Some(_) => {}
                    None => output = Some(diagnostic_output.clone()),
                }
            }
            let parsed = step
                .diagnostic_format
                .zip(
                    diagnostic_output
                        .map(String::as_str)
                        .or(output.as_deref())
                        .filter(|output| !output.is_empty()),
                )
                .map(|(diagnostic_format, output)| {
                    diagnostics::parse(
                        diagnostic_format,
                        name,
                        step.diagnostic_tool.as_deref().unwrap_or(name),
                        output,
                    )
                })
                .unwrap_or_default();
            steps.push(StepResult {
                name: name.clone(),
                status,
                failure_allowed: allowed_failures.contains(name),
                duration_ms: timings.get(name).copied().unwrap_or(0),
                effects: executed_effects
                    .get(name)
                    .into_iter()
                    .flatten()
                    .map(|(command, effect)| ExecutedEffect {
                        command: command.clone(),
                        effect: *effect,
                    })
                    .collect(),
                diagnostics: parsed.diagnostics,
                parse_warnings: parsed.warnings,
                output_kind,
                output,
                skip_reason,
            });
        }
    }
    drop(outputs);
    drop(diagnostic_outputs);
    drop(cancelled);
    drop(finished);
    drop(failed);
    drop(allowed_failures);

    let result = RunResult {
        schema_version: 1,
        kind: "run_result",
        hook: hook.to_string(),
        status: run_status(failure.as_deref(), run_was_cancelled),
        started_at,
        duration_ms,
        failure,
        reason: None,
        steps,
    };
    if format != OutputFormat::Human {
        emit_result(format, &result)?;
    }
    if let Some(path) = reports.sarif {
        let diagnostics = result
            .steps
            .iter()
            .flat_map(|step| step.diagnostics.iter().cloned())
            .collect::<Vec<_>>();
        diagnostics::write_sarif(path, &diagnostics)?;
    }
    if let Some(path) = reports.junit {
        write_junit(path, &result)?;
    }
    Ok(())
}

/// Emit a complete machine-readable result for a successful run that did not
/// start any commands (for example, because there were no matching files).
pub fn emit_noop_run(
    format: OutputFormat,
    hook: &str,
    started_at: String,
    duration_ms: u128,
    steps: Vec<(String, String)>,
    reason: &str,
    reports: ReportPaths,
) -> Result<()> {
    let result = RunResult {
        schema_version: 1,
        kind: "run_result",
        hook: hook.to_string(),
        status: "passed",
        started_at,
        duration_ms,
        failure: None,
        reason: Some(reason.to_string()),
        steps: steps
            .into_iter()
            .map(|(name, skip_reason)| StepResult {
                name,
                status: "skipped",
                failure_allowed: false,
                duration_ms: 0,
                effects: vec![],
                diagnostics: vec![],
                parse_warnings: vec![],
                output_kind: None,
                output: None,
                skip_reason: Some(skip_reason),
            })
            .collect(),
    };
    if format != OutputFormat::Human {
        emit_result(format, &result)?;
    }
    if let Some(path) = reports.sarif {
        diagnostics::write_sarif(path, &[])?;
    }
    if let Some(path) = reports.junit {
        write_junit(path, &result)?;
    }
    Ok(())
}

pub fn emit_error_run(
    format: OutputFormat,
    hook: &str,
    started_at: String,
    duration_ms: u128,
    failure: String,
    reports: ReportPaths,
) -> Result<()> {
    let result = RunResult {
        schema_version: 1,
        kind: "run_result",
        hook: hook.to_string(),
        status: "failed",
        started_at,
        duration_ms,
        failure: Some(failure),
        reason: None,
        steps: vec![],
    };
    if format != OutputFormat::Human {
        emit_result(format, &result)?;
    }
    if let Some(path) = reports.sarif {
        diagnostics::write_sarif(path, &[])?;
    }
    if let Some(path) = reports.junit {
        write_junit(path, &result)?;
    }
    Ok(())
}

fn emit_result(format: OutputFormat, result: &RunResult) -> Result<()> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    match format {
        OutputFormat::Human => {}
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut stdout, result)?;
            writeln!(stdout)?;
        }
        OutputFormat::Jsonl => {
            drop(stdout);
            write_jsonl_event("run_completed", result)?;
        }
    }
    Ok(())
}

fn write_jsonl_event(event: &str, data: impl Serialize) -> Result<()> {
    let mut sequence = jsonl_sequence().lock().unwrap();
    write_event(
        &mut std::io::stdout().lock(),
        &Event {
            schema_version: 1,
            event,
            sequence: *sequence,
            data,
        },
    )?;
    *sequence += 1;
    Ok(())
}

fn write_event(writer: &mut impl Write, event: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *writer, event)?;
    writeln!(writer)?;
    Ok(())
}

/// Write the run result as a JUnit XML report, treating each hk step as a
/// single test case. This deliberately does not parse tool-specific output —
/// it only reports the status, duration, and captured stdout/stderr that hk
/// already tracks for every step. See
/// <https://github.com/jdx/hk/discussions/1215>.
fn write_junit(path: &Path, run: &RunResult) -> Result<()> {
    use std::fmt::Write as _;

    let classname = format!("hk.{}", run.hook);
    let mut cases = String::new();
    let mut failures = 0usize;
    let mut errors = 0usize;
    let mut skipped = 0usize;

    for step in &run.steps {
        let time = step.duration_ms as f64 / 1000.0;
        writeln!(
            cases,
            "    <testcase name=\"{}\" classname=\"{}\" time=\"{:.3}\">",
            xml_escape(&step.name),
            xml_escape(&classname),
            time,
        )
        .unwrap();
        if step.failure_allowed {
            cases.push_str(
                "      <properties><property name=\"failure_allowed\" value=\"true\"/></properties>\n",
            );
        }
        match step.status {
            // hk treats an allowed failure as accepted and the run succeeds
            // despite it, but JUnit has no such concept: any <failure> makes
            // standard consumers report the run failed regardless of the
            // failure_allowed property, so this must not count as a failure.
            "failed" if step.failure_allowed => {
                if let Some(output) = step.output.as_deref().filter(|o| !o.is_empty()) {
                    writeln!(
                        cases,
                        "      <system-out>{}</system-out>",
                        xml_escape(output)
                    )
                    .unwrap();
                }
            }
            "failed" => {
                failures += 1;
                writeln!(
                    cases,
                    "      <failure message=\"step failed\">{}</failure>",
                    xml_escape(step.output.as_deref().unwrap_or_default()),
                )
                .unwrap();
            }
            "cancelled" => {
                skipped += 1;
                writeln!(
                    cases,
                    "      <skipped message=\"run cancelled before step completed\"/>",
                )
                .unwrap();
            }
            "skipped" => {
                skipped += 1;
                writeln!(
                    cases,
                    "      <skipped message=\"{}\"/>",
                    xml_escape(step.skip_reason.as_deref().unwrap_or("skipped")),
                )
                .unwrap();
            }
            _ => {
                if let Some(output) = step.output.as_deref().filter(|o| !o.is_empty()) {
                    writeln!(
                        cases,
                        "      <system-out>{}</system-out>",
                        xml_escape(output)
                    )
                    .unwrap();
                }
            }
        }
        cases.push_str("    </testcase>\n");
    }

    // A run can fail without any step reporting it: before any step is
    // planned (for example, `git status` failing during setup), or after
    // every step finishes (for example, `fail_on_fix` rejecting a fix that
    // modified files, or a stash restore failing). Without a synthetic test
    // case here, a JUnit consumer would see zero failures/errors and treat
    // the report as passing, hiding a real failure.
    let mut tests = run.steps.len();
    if failures == 0
        && let Some(failure) = &run.failure
    {
        tests += 1;
        errors += 1;
        writeln!(
            cases,
            "    <testcase name=\"{}\" classname=\"hk\" time=\"{:.3}\">\n      <error message=\"{}\"/>\n    </testcase>",
            xml_escape(&run.hook),
            run.duration_ms as f64 / 1000.0,
            xml_escape(failure),
        )
        .unwrap();
    }

    let time = run.duration_ms as f64 / 1000.0;
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    writeln!(
        xml,
        "<testsuites name=\"hk\" tests=\"{tests}\" failures=\"{failures}\" errors=\"{errors}\" skipped=\"{skipped}\" time=\"{time:.3}\">"
    )
    .unwrap();
    writeln!(
        xml,
        "  <testsuite name=\"{}\" tests=\"{tests}\" failures=\"{failures}\" errors=\"{errors}\" skipped=\"{skipped}\" time=\"{time:.3}\" timestamp=\"{}\">",
        xml_escape(&run.hook),
        xml_escape(&run.started_at),
    )
    .unwrap();
    xml.push_str(&cases);
    xml.push_str("  </testsuite>\n</testsuites>\n");

    xx::file::write(path, xml.as_bytes())?;
    Ok(())
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // Strip control characters that are not valid in XML 1.0 text.
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' && c != '\r' => {}
            c => out.push(c),
        }
    }
    out
}

fn run_status(failure: Option<&str>, cancelled: bool) -> &'static str {
    if failure.is_some() {
        "failed"
    } else if cancelled {
        "cancelled"
    } else {
        "passed"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_a_top_level_terminal_status() {
        assert_eq!(run_status(None, true), "cancelled");
        assert_eq!(run_status(Some("step failed"), true), "failed");
        assert_eq!(run_status(None, false), "passed");
    }

    fn step(name: &str, status: &'static str) -> StepResult {
        StepResult {
            name: name.to_string(),
            status,
            failure_allowed: false,
            duration_ms: 10,
            effects: vec![],
            diagnostics: vec![],
            parse_warnings: vec![],
            output_kind: None,
            output: None,
            skip_reason: None,
        }
    }

    #[test]
    fn junit_report_counts_each_step_status() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junit.xml");
        let mut failed = step("lint", "failed");
        failed.output = Some("boom <bad>".to_string());
        let mut skipped = step("format", "skipped");
        skipped.skip_reason = Some("no matching files".to_string());
        let cancelled = step("clippy", "cancelled");
        let run = RunResult {
            schema_version: 1,
            kind: "run_result",
            hook: "check".to_string(),
            status: "failed",
            started_at: "2024-01-01T00:00:00Z".to_string(),
            duration_ms: 250,
            failure: Some("step failed".to_string()),
            reason: None,
            steps: vec![step("build", "passed"), failed, skipped, cancelled],
        };
        write_junit(&path, &run).unwrap();
        let xml = std::fs::read_to_string(&path).unwrap();

        assert!(xml.contains(
            "<testsuite name=\"check\" tests=\"4\" failures=\"1\" errors=\"0\" skipped=\"2\""
        ));
        assert!(xml.contains("<testcase name=\"lint\""));
        // The captured output is XML-escaped rather than injected raw.
        assert!(xml.contains("boom &lt;bad&gt;"));
        assert!(xml.contains("<skipped message=\"no matching files\"/>"));
        // A step cancelled because a sibling failed never ran to completion,
        // so it's reported as skipped rather than as a failure/error.
        assert!(xml.contains("<skipped message=\"run cancelled before step completed\"/>"));
    }

    #[test]
    fn junit_report_adds_synthetic_case_for_setup_failures() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junit.xml");
        let run = RunResult {
            schema_version: 1,
            kind: "run_result",
            hook: "check".to_string(),
            status: "failed",
            started_at: "2024-01-01T00:00:00Z".to_string(),
            duration_ms: 5,
            failure: Some("git status failed".to_string()),
            reason: None,
            steps: vec![],
        };
        write_junit(&path, &run).unwrap();
        let xml = std::fs::read_to_string(&path).unwrap();

        assert!(xml.contains("tests=\"1\" failures=\"0\" errors=\"1\""));
        assert!(xml.contains("<error message=\"git status failed\"/>"));
    }

    #[test]
    fn junit_report_adds_synthetic_case_for_post_step_failures() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junit.xml");
        let run = RunResult {
            schema_version: 1,
            kind: "run_result",
            hook: "fix".to_string(),
            status: "failed",
            started_at: "2024-01-01T00:00:00Z".to_string(),
            duration_ms: 20,
            // e.g. `fail_on_fix` rejecting a fix, or a stash restore failing
            // after every step already finished successfully.
            failure: Some("fix modified files, aborting".to_string()),
            reason: None,
            steps: vec![step("format", "passed")],
        };
        write_junit(&path, &run).unwrap();
        let xml = std::fs::read_to_string(&path).unwrap();

        assert!(xml.contains("tests=\"2\" failures=\"0\" errors=\"1\""));
        assert!(xml.contains("<error message=\"fix modified files, aborting\"/>"));
    }

    #[test]
    fn junit_report_does_not_count_allowed_failures() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("junit.xml");
        let mut allowed = step("flaky", "failed");
        allowed.failure_allowed = true;
        allowed.output = Some("transient error".to_string());
        let run = RunResult {
            schema_version: 1,
            kind: "run_result",
            hook: "check".to_string(),
            status: "passed",
            started_at: "2024-01-01T00:00:00Z".to_string(),
            duration_ms: 10,
            failure: None,
            reason: None,
            steps: vec![allowed],
        };
        write_junit(&path, &run).unwrap();
        let xml = std::fs::read_to_string(&path).unwrap();

        assert!(xml.contains("tests=\"1\" failures=\"0\" errors=\"0\" skipped=\"0\""));
        assert!(xml.contains("<system-out>transient error</system-out>"));
        assert!(!xml.contains("<failure"));
    }
}
