//! Stop-hook runner for coding agents.
//!
//! Claude Code and Codex both treat a Stop hook's stdout as a JSON decision and its exit code as
//! a separate signal, so a raw `hk run check` hook leaks hk's own result JSON and linter exit
//! codes into the agent. This runner owns the whole contract: it always exits 0 and its only
//! possible output is `{"decision":"block","reason":"..."}`, or when the check passes nothing
//! (Claude Code's documented pass) or `{}` for Codex, whose docs contradict themselves about
//! empty stdout; `{}` is valid under either reading.

use std::io::{IsTerminal, Read};
use std::process::Output;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::AsyncReadExt;

use serde_json::{Value, json};

const MAX_REASON_CHARS: usize = 2000;
const MAX_STEP_OUTPUT_CHARS: usize = 600;
const MAX_DIAGNOSTICS: usize = 10;
/// Default deadline for the check. Codex stops waiting for the hook after 120 seconds (see
/// `hk agent hooks --target codex`), so give up a little earlier. The Claude Code snippet
/// sets its own, longer `--timeout` together with a matching hook `timeout`.
const DEFAULT_CHECK_TIMEOUT: Duration = Duration::from_secs(100);
/// How long to keep reading output after the check has exited.
const PIPE_DRAIN_GRACE: Duration = Duration::from_secs(5);

/// Most bytes kept per output stream; a check can print far more than a block reason needs.
const MAX_CAPTURED_BYTES: usize = 1024 * 1024;

/// Reads a stream to the end but keeps at most `cap` bytes: the head for stdout (the start
/// of the JSON document), the tail for stderr (where a failure is reported).
///
/// What was read so far lives in `sink`, so it survives the reader being aborted while a
/// descendant of the check still holds the pipe open.
async fn read_capped<R: AsyncReadExt + Unpin>(
    mut reader: R,
    cap: usize,
    keep_tail: bool,
    sink: Arc<Mutex<Vec<u8>>>,
) {
    let mut buffer = [0_u8; 8192];
    loop {
        let count = match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        let mut kept = sink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if keep_tail {
            kept.extend_from_slice(&buffer[..count]);
            if kept.len() > cap * 2 {
                let excess = kept.len() - cap;
                kept.drain(..excess);
            }
        } else {
            let room = cap.saturating_sub(kept.len());
            kept.extend_from_slice(&buffer[..count.min(room)]);
        }
    }
}

/// The bytes a [`read_capped`] sink holds, trimmed to the tail `cap` when `keep_tail`.
fn captured(sink: &Mutex<Vec<u8>>, cap: usize, keep_tail: bool) -> Vec<u8> {
    let mut kept = sink.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut bytes = std::mem::take(&mut *kept);
    if keep_tail && bytes.len() > cap {
        bytes.drain(..bytes.len() - cap);
    }
    bytes
}

enum CheckOutcome {
    Finished(Output),
    TimedOut,
    /// The agent terminated the hook; nothing may be printed.
    Interrupted,
}

/// Runs the check in its own process group and kills the whole group on a timeout or when
/// the hook itself is terminated, so no check keeps running after the hook stops waiting.
async fn run_check(
    mut command: tokio::process::Command,
    timeout: Duration,
) -> std::io::Result<CheckOutcome> {
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let stdout_sink = Arc::new(Mutex::new(Vec::new()));
    let stderr_sink = Arc::new(Mutex::new(Vec::new()));
    let stdout_task = tokio::spawn(read_capped(
        stdout,
        MAX_CAPTURED_BYTES,
        false,
        stdout_sink.clone(),
    ));
    let stderr_task = tokio::spawn(read_capped(
        stderr,
        MAX_CAPTURED_BYTES,
        true,
        stderr_sink.clone(),
    ));
    let status = tokio::select! {
        status = child.wait() => Some(status?),
        _ = tokio::time::sleep(timeout) => None,
        _ = terminated() => {
            kill_group(&mut child).await;
            return Ok(CheckOutcome::Interrupted);
        }
    };
    let Some(status) = status else {
        kill_group(&mut child).await;
        return Ok(CheckOutcome::TimedOut);
    };
    // A step that outlives the check can hold a pipe open; do not wait for it forever.
    // Whatever was read before the wait gave up is still reported.
    for task in [stdout_task, stderr_task] {
        let abort = task.abort_handle();
        if tokio::time::timeout(PIPE_DRAIN_GRACE, task).await.is_err() {
            abort.abort();
        }
    }
    let stdout = captured(&stdout_sink, MAX_CAPTURED_BYTES, false);
    let stderr = captured(&stderr_sink, MAX_CAPTURED_BYTES, true);
    Ok(CheckOutcome::Finished(Output {
        status,
        stdout,
        stderr,
    }))
}

async fn kill_group(child: &mut tokio::process::Child) {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // SAFETY: the child leads its own process group (`process_group(0)`), so this
        // signals only the check and the processes it started.
        unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGKILL) };
    }
    #[cfg(windows)]
    if let Some(pid) = child.id() {
        // Windows has no process groups: kill the check's whole tree while the check is
        // still alive, so the linters it spawned do not outlive the hook.
        let _ = tokio::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

/// Resolves when the agent terminates or interrupts the hook.
async fn terminated() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let (Ok(mut term), Ok(mut int)) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        ) else {
            return std::future::pending().await;
        };
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Whether the agent says this stop already continued because of a Stop hook.
fn stop_hook_active(input: &str) -> bool {
    serde_json::from_str::<Value>(input)
        .ok()
        .and_then(|v| v.get("stop_hook_active").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn truncate(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() <= MAX_REASON_CHARS {
        return text.to_string();
    }
    let mut out: String = text.chars().take(MAX_REASON_CHARS).collect();
    out.push_str("...");
    out
}

/// Parses hk's JSON result, or, when the capture was cut off (see `MAX_CAPTURED_BYTES`),
/// whatever complete values precede the cut. The scanner tracks JSON strings and nesting,
/// remembers the last point where a value was complete, and closes the open containers
/// there, so a long step `output` cut mid-string costs only that field.
fn parse_result(text: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    let bytes = text.as_bytes();
    // Per open container: `{` or `[`, and for objects whether a key is expected next.
    let mut stack: Vec<(u8, bool)> = Vec::new();
    let mut safe: Option<(usize, Vec<u8>)> = None;
    let mut mark = |end: usize, stack: &[(u8, bool)]| {
        safe = Some((end, stack.iter().map(|(kind, _)| *kind).collect()));
    };
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let mut j = i + 1;
                loop {
                    match bytes.get(j) {
                        None => break,
                        Some(b'\\') => j += 2,
                        Some(b'"') => break,
                        Some(_) => j += 1,
                    }
                }
                if j >= bytes.len() {
                    break; // cut off inside a string
                }
                match stack.last_mut() {
                    Some((b'{', expect_key)) if *expect_key => *expect_key = false,
                    _ => mark(j + 1, &stack),
                }
                i = j + 1;
            }
            kind @ (b'{' | b'[') => {
                stack.push((kind, kind == b'{'));
                i += 1;
            }
            b'}' | b']' => {
                stack.pop()?;
                mark(i + 1, &stack);
                i += 1;
            }
            b',' => {
                if let Some((b'{', expect_key)) = stack.last_mut() {
                    *expect_key = true;
                }
                i += 1;
            }
            b':' | b' ' | b'\n' | b'\r' | b'\t' => i += 1,
            _ => {
                // number or literal: complete only when a delimiter follows it
                let start = i;
                while i < bytes.len() && !b",]} \n\r\t".contains(&bytes[i]) {
                    i += 1;
                }
                if i < bytes.len() && start < i {
                    mark(i, &stack);
                }
            }
        }
    }
    let (end, open) = safe?;
    let mut repaired = text[..end].to_string();
    for kind in open.iter().rev() {
        repaired.push(if *kind == b'{' { '}' } else { ']' });
    }
    serde_json::from_str(&repaired).ok()
}

/// Build a short, human-readable explanation from the child's JSON result and stderr.
fn diagnose(stdout: &str, stderr: &str, code: Option<i32>) -> String {
    let mut lines = vec![];
    if let Some(result) = parse_result(stdout) {
        for key in ["failure", "reason"] {
            if let Some(text) = result.get(key).and_then(Value::as_str) {
                lines.push(text.trim().to_string());
            }
        }
        let diagnostics: Vec<&Value> = result
            .get("steps")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|step| step.get("diagnostics").and_then(Value::as_array))
            .flatten()
            .collect();
        for d in diagnostics.iter().take(MAX_DIAGNOSTICS) {
            let field = |name: &str| d.get(name).and_then(Value::as_str);
            let location = field("path").map(|p| format!("{p}: ")).unwrap_or_default();
            lines.push(format!(
                "{location}{} ({})",
                field("message").unwrap_or("issue"),
                field("step").unwrap_or("step")
            ));
        }
        if diagnostics.len() > MAX_DIAGNOSTICS {
            lines.push(format!(
                "...and {} more",
                diagnostics.len() - MAX_DIAGNOSTICS
            ));
        }
        // Failed steps without parsed diagnostics: show their raw output.
        for step in result
            .get("steps")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let no_diagnostics = step
                .get("diagnostics")
                .and_then(Value::as_array)
                .is_none_or(|d| d.is_empty());
            if step.get("status").and_then(Value::as_str) != Some("failed") || !no_diagnostics {
                continue;
            }
            let name = step.get("name").and_then(Value::as_str).unwrap_or("step");
            let output = step
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            let output: String = output.chars().take(MAX_STEP_OUTPUT_CHARS).collect();
            if output.is_empty() {
                lines.push(format!("{name} failed"));
            } else {
                lines.push(format!("{name} failed:\n{output}"));
            }
        }
    }
    // Drop the source location hk appends to errors; it is noise for an agent.
    let stderr = stderr
        .split("\n\nLocation:")
        .next()
        .unwrap_or(stderr)
        .trim();
    if lines.is_empty() && !stderr.is_empty() {
        lines.push(stderr.to_string());
    }
    let status = code.map_or("was terminated by a signal".to_string(), |c| {
        format!("exited with status {c}")
    });
    let mut reason = format!("hk check {status}.");
    if !lines.is_empty() {
        reason.push('\n');
        reason.push_str(&lines.join("\n"));
    }
    truncate(&reason)
}

/// What a passing check prints: `{}` for Codex, nothing for Claude Code.
fn pass_output(codex: bool) -> Option<&'static str> {
    codex.then_some("{}")
}

fn block(reason: &str) -> String {
    json!({"decision": "block", "reason": reason}).to_string()
}

pub async fn run(timeout: Option<Duration>, codex: bool) -> crate::Result<()> {
    let timeout = timeout.unwrap_or(DEFAULT_CHECK_TIMEOUT);
    let mut input = String::new();
    if !std::io::stdin().is_terminal() {
        let _ = std::io::stdin().read_to_string(&mut input);
    }
    if stop_hook_active(&input) {
        if let Some(pass) = pass_output(codex) {
            println!("{pass}");
        }
        return Ok(());
    }
    let output = match std::env::current_exe() {
        Ok(exe) => {
            let mut command = tokio::process::Command::new(exe);
            // Trace records would end up in the output this hook parses.
            command
                .args(["run", "check", "--safe", "--format", "json"])
                .env_remove("HK_TRACE");
            run_check(command, timeout).await
        }
        Err(err) => Err(err),
    };
    match output {
        Ok(CheckOutcome::Finished(output)) if output.status.success() => {
            if let Some(pass) = pass_output(codex) {
                println!("{pass}");
            }
        }
        Ok(CheckOutcome::Interrupted) => {}
        Ok(CheckOutcome::TimedOut) => println!(
            "{}",
            block(&format!(
                "hk check did not finish within {} seconds and was stopped.",
                timeout.as_secs()
            ))
        ),
        Ok(CheckOutcome::Finished(output)) => println!(
            "{}",
            block(&diagnose(
                &String::from_utf8_lossy(&output.stdout),
                &String::from_utf8_lossy(&output.stderr),
                output.status.code(),
            ))
        ),
        Err(err) => println!(
            "{}",
            block(&truncate(&format!("failed to run hk check: {err}")))
        ),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_active_flag() {
        assert!(stop_hook_active(r#"{"stop_hook_active":true}"#));
        assert!(!stop_hook_active(r#"{"stop_hook_active":false}"#));
        assert!(!stop_hook_active("not json"));
        assert!(!stop_hook_active(""));
    }

    #[test]
    fn diagnoses_refusal_and_diagnostics() {
        let out = r#"{"status":"failed","failure":"--safe refused to run: x","steps":[{"name":"a","status":"failed","diagnostics":[{"step":"a","message":"bad","path":"f.rs"}]}]}"#;
        let reason = diagnose(out, "", Some(1));
        assert!(reason.contains("--safe refused to run: x"));
        assert!(reason.contains("f.rs: bad (a)"));
    }

    #[test]
    fn a_result_cut_off_inside_a_huge_step_output_still_explains_the_failure() {
        let big = "x".repeat(2 * MAX_CAPTURED_BYTES);
        let document = serde_json::json!({
            "status": "failed",
            "failure": "--safe refused to run: probe is unknown",
            "steps": [
                {"name": "ok", "status": "succeeded", "diagnostics": [], "output": "fine"},
                {"name": "lint", "status": "failed", "diagnostics": [], "output": big},
            ],
        });
        let pretty = serde_json::to_string_pretty(&document).unwrap();
        assert!(pretty.len() > 2 * MAX_CAPTURED_BYTES);
        // Cut mid-string, as the capture limit does.
        let head = &pretty[..MAX_CAPTURED_BYTES];
        assert!(serde_json::from_str::<Value>(head).is_err());
        let reason = diagnose(head, "", Some(1));
        assert!(reason.contains("--safe refused to run: probe is unknown"));
        assert!(reason.contains("lint failed"), "{reason}");
    }

    #[test]
    fn truncated_results_are_repaired_at_the_last_complete_value() {
        let value = parse_result(r#"{"a":1,"b":[{"c":"d"},{"e":"unfinished"#).unwrap();
        assert_eq!(value, serde_json::json!({"a": 1, "b": [{"c": "d"}]}));
        let value = parse_result(r#"{"a":"x","b":12"#).unwrap();
        assert_eq!(value, serde_json::json!({"a": "x"}));
        assert!(parse_result("not json").is_none());
    }

    #[test]
    fn falls_back_to_stderr_and_truncates() {
        assert!(diagnose("", "boom", Some(2)).contains("boom"));
        assert!(diagnose("", &"x".repeat(5000), None).chars().count() <= MAX_REASON_CHARS + 3);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_timed_out_check_is_killed_with_its_steps() {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "sleep 300 & wait"]);
        let started = std::time::Instant::now();
        let outcome = run_check(command, Duration::from_millis(300))
            .await
            .unwrap();
        assert!(matches!(outcome, CheckOutcome::TimedOut));
        // The step's `sleep` is gone too, so nothing holds the pipes or keeps working.
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn a_timed_out_check_is_killed_with_its_steps_on_windows() {
        // cmd starts a child `ping` (about 30s) and waits for it; the tree must die promptly.
        let mut command = tokio::process::Command::new("cmd");
        command.args(["/C", "ping -n 30 127.0.0.1 >NUL"]);
        let started = std::time::Instant::now();
        let outcome = run_check(command, Duration::from_millis(500))
            .await
            .unwrap();
        assert!(matches!(outcome, CheckOutcome::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[tokio::test]
    async fn captured_output_is_capped_keeping_the_head_or_the_tail() {
        let data = (0..100_000_u32)
            .flat_map(|n| n.to_le_bytes())
            .collect::<Vec<u8>>();
        let read = async |bytes: &[u8], keep_tail: bool| {
            let sink = Arc::new(Mutex::new(Vec::new()));
            read_capped(bytes, 1000, keep_tail, sink.clone()).await;
            captured(&sink, 1000, keep_tail)
        };
        assert_eq!(read(&data, false).await, data[..1000]);
        assert_eq!(read(&data, true).await, data[data.len() - 1000..]);
        assert_eq!(read(&data[..10], true).await, data[..10]);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn output_read_before_a_lingering_pipe_holder_is_still_returned() {
        // The check exits, but a background process keeps stdout open.
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "echo partial; sleep 30 & exit 1"]);
        let started = std::time::Instant::now();
        let CheckOutcome::Finished(output) =
            run_check(command, Duration::from_secs(60)).await.unwrap()
        else {
            panic!("expected the check to finish");
        };
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"partial\n");
        assert!(started.elapsed() < Duration::from_secs(20));
        let reason = diagnose(&String::from_utf8_lossy(&output.stdout), "boom", Some(1));
        assert!(reason.contains("boom"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_finished_check_returns_its_output() {
        let mut command = tokio::process::Command::new("sh");
        command.args(["-c", "echo out; echo err >&2; exit 3"]);
        let CheckOutcome::Finished(output) =
            run_check(command, Duration::from_secs(30)).await.unwrap()
        else {
            panic!("expected the check to finish");
        };
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(output.stdout, b"out\n");
        assert_eq!(output.stderr, b"err\n");
    }

    #[test]
    fn a_passing_check_prints_braces_only_for_codex() {
        assert_eq!(pass_output(true), Some("{}"));
        assert_eq!(pass_output(false), None);
    }

    #[test]
    fn block_is_single_line_json() {
        let text = block("a\nb");
        assert!(!text.contains('\n'));
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap()["decision"],
            "block"
        );
    }
}
