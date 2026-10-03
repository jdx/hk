//! Stop-hook runner for coding agents.
//!
//! Claude Code and Codex both treat a Stop hook's stdout as a JSON decision and its exit code as
//! a separate signal, so a raw `hk run check` hook leaks hk's own result JSON and linter exit
//! codes into the agent. This runner owns the whole contract: it always exits 0 and its only
//! possible output is `{"decision":"block","reason":"..."}`.

use std::io::{IsTerminal, Read};
use std::process::Output;
use std::time::Duration;

use tokio::io::AsyncReadExt;

use serde_json::{Value, json};

const MAX_REASON_CHARS: usize = 2000;
const MAX_STEP_OUTPUT_CHARS: usize = 600;
const MAX_DIAGNOSTICS: usize = 10;
/// Codex stops waiting for the hook after 120 seconds (see `hk agent hooks --target codex`);
/// give up a little earlier so the check never outlives the hook.
const CHECK_TIMEOUT: Duration = Duration::from_secs(100);
/// How long to keep reading output after the check has exited.
const PIPE_DRAIN_GRACE: Duration = Duration::from_secs(5);

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
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let stdout_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes).await;
        bytes
    });
    let stderr_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes).await;
        bytes
    });
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
    let collect = async |task: tokio::task::JoinHandle<Vec<u8>>| {
        let abort = task.abort_handle();
        match tokio::time::timeout(PIPE_DRAIN_GRACE, task).await {
            Ok(bytes) => bytes.unwrap_or_default(),
            Err(_) => {
                abort.abort();
                Vec::new()
            }
        }
    };
    let stdout = collect(stdout_task).await;
    let stderr = collect(stderr_task).await;
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

/// Build a short, human-readable explanation from the child's JSON result and stderr.
fn diagnose(stdout: &str, stderr: &str, code: Option<i32>) -> String {
    let mut lines = vec![];
    if let Ok(result) = serde_json::from_str::<Value>(stdout) {
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

fn block(reason: &str) -> String {
    json!({"decision": "block", "reason": reason}).to_string()
}

pub async fn run() -> crate::Result<()> {
    let mut input = String::new();
    if !std::io::stdin().is_terminal() {
        let _ = std::io::stdin().read_to_string(&mut input);
    }
    if stop_hook_active(&input) {
        return Ok(());
    }
    let output = match std::env::current_exe() {
        Ok(exe) => {
            let mut command = tokio::process::Command::new(exe);
            // Trace records would end up in the output this hook parses.
            command
                .args(["run", "check", "--safe", "--format", "json"])
                .env_remove("HK_TRACE");
            run_check(command, CHECK_TIMEOUT).await
        }
        Err(err) => Err(err),
    };
    match output {
        Ok(CheckOutcome::Finished(output)) if output.status.success() => {}
        Ok(CheckOutcome::Interrupted) => {}
        Ok(CheckOutcome::TimedOut) => println!(
            "{}",
            block(&format!(
                "hk check did not finish within {} seconds and was stopped.",
                CHECK_TIMEOUT.as_secs()
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
    fn block_is_single_line_json() {
        let text = block("a\nb");
        assert!(!text.contains('\n'));
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap()["decision"],
            "block"
        );
    }
}
