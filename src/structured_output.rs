use crate::{
    Result,
    diagnostics::{self, Diagnostic},
    hook::HookContext,
    step::{CommandEffect, OutputSummary},
};
use serde::Serialize;
use std::io::Write;
use std::path::Path;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Eq,
    PartialEq,
    clap::ValueEnum,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
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

pub fn emit_run(
    format: OutputFormat,
    hook: &str,
    started_at: String,
    duration_ms: u128,
    ctx: &HookContext,
    failure: Option<String>,
    sarif_path: Option<&Path>,
) -> Result<()> {
    let failed = ctx.failed_steps.lock().unwrap();
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
            let status = if failed.contains(name) {
                "failed"
            } else if skip_reason.is_some() {
                "skipped"
            } else if cancelled.contains(name) {
                "cancelled"
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
                    Some(output) => output.insert_str(0, diagnostic_output),
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
    if let Some(path) = sarif_path {
        let diagnostics = result
            .steps
            .iter()
            .flat_map(|step| step.diagnostics.iter().cloned())
            .collect::<Vec<_>>();
        diagnostics::write_sarif(path, &diagnostics)?;
    }
    if format == OutputFormat::Human {
        return Ok(());
    }
    emit_result(format, &result)
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
    sarif_path: Option<&Path>,
) -> Result<()> {
    if let Some(path) = sarif_path {
        diagnostics::write_sarif(path, &[])?;
    }
    if format == OutputFormat::Human {
        return Ok(());
    }
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
    emit_result(format, &result)
}

pub fn emit_error_run(
    format: OutputFormat,
    hook: &str,
    started_at: String,
    duration_ms: u128,
    failure: String,
    sarif_path: Option<&Path>,
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
    if let Some(path) = sarif_path {
        diagnostics::write_sarif(path, &[])?;
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
            write_event(
                &mut stdout,
                &Event {
                    schema_version: 1,
                    event: "run_started",
                    sequence: 0,
                    data: serde_json::json!({
                        "hook": result.hook,
                        "started_at": result.started_at,
                    }),
                },
            )?;
            for (index, step) in result.steps.iter().enumerate() {
                write_event(
                    &mut stdout,
                    &Event {
                        schema_version: 1,
                        event: "step_completed",
                        sequence: index + 1,
                        data: step,
                    },
                )?;
            }
            write_event(
                &mut stdout,
                &Event {
                    schema_version: 1,
                    event: "run_completed",
                    sequence: result.steps.len() + 1,
                    data: serde_json::json!({
                        "hook": result.hook,
                        "status": result.status,
                        "duration_ms": result.duration_ms,
                        "failure": result.failure,
                        "reason": result.reason,
                    }),
                },
            )?;
        }
    }
    Ok(())
}

fn write_event(writer: &mut impl Write, event: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *writer, event)?;
    writeln!(writer)?;
    Ok(())
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
    use super::run_status;

    #[test]
    fn cancellation_is_a_top_level_terminal_status() {
        assert_eq!(run_status(None, true), "cancelled");
        assert_eq!(run_status(Some("step failed"), true), "failed");
        assert_eq!(run_status(None, false), "passed");
    }
}
