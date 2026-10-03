//! Stop-hook runner for coding agents.
//!
//! Claude Code and Codex both treat a Stop hook's stdout as a JSON decision and its exit code as
//! a separate signal, so a raw `hk run check` hook leaks hk's own result JSON and linter exit
//! codes into the agent. This runner owns the whole contract: it always exits 0 and its only
//! possible output is `{"decision":"block","reason":"..."}`.

use std::io::{IsTerminal, Read};

use serde_json::{Value, json};

const MAX_REASON_CHARS: usize = 2000;
const MAX_STEP_OUTPUT_CHARS: usize = 600;
const MAX_DIAGNOSTICS: usize = 10;

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
            tokio::process::Command::new(exe)
                .args(["run", "check", "--safe", "--format", "json"])
                .stdin(std::process::Stdio::null())
                .output()
                .await
        }
        Err(err) => Err(err),
    };
    match output {
        Ok(output) if output.status.success() => {}
        Ok(output) => println!(
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
