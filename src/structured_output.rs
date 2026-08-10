use crate::{Result, hook::HookContext, step::OutputSummary};
use serde::Serialize;
use std::io::Write;

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
    steps: Vec<StepResult>,
}

#[derive(Debug, Clone, Serialize)]
struct StepResult {
    name: String,
    status: &'static str,
    duration_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_kind: Option<OutputSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skip_reason: Option<String>,
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
    ctx: &HookContext,
    succeeded: bool,
) -> Result<()> {
    if format == OutputFormat::Human {
        return Ok(());
    }

    let failed = ctx.failed_steps.lock().unwrap();
    let skipped = ctx.get_skipped_steps();
    let outputs = ctx.output_by_step.lock().unwrap();
    let timings = ctx.timing.step_wall_times();
    let mut steps = Vec::new();
    for group in &ctx.groups {
        for name in group.steps.keys() {
            let skip_reason = skipped.get(name).map(|reason| reason.message());
            let status = if failed.contains(name) {
                "failed"
            } else if skip_reason.is_some() {
                "skipped"
            } else {
                "passed"
            };
            let (output_kind, output) = outputs
                .get(name)
                .map(|(kind, output)| (Some(kind.clone()), Some(output.clone())))
                .unwrap_or((None, None));
            steps.push(StepResult {
                name: name.clone(),
                status,
                duration_ms: timings.get(name).copied().unwrap_or(0),
                output_kind,
                output,
                skip_reason,
            });
        }
    }
    drop(outputs);
    drop(failed);

    let result = RunResult {
        schema_version: 1,
        kind: "run_result",
        hook: hook.to_string(),
        status: if succeeded { "passed" } else { "failed" },
        started_at,
        duration_ms: ctx.timing.elapsed_ms(),
        steps,
    };
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    match format {
        OutputFormat::Human => {}
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut stdout, &result)?;
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
