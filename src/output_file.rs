//! The output file (`HK_OUTPUT_FILE`, `output.log` by default) holds the step
//! summaries of the latest hook run, so a reader without the console (a coding
//! agent, a CI artifact) sees every step's output, not only the first failure.

use crate::env;
use std::{
    io::Write,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// Whether this process wrote the output file from a hook run's summaries.
static WRITTEN: AtomicBool = AtomicBool::new(false);

/// Output of every failed step command. A command that ends the process with
/// one of these belongs to a step, whose summary (rendered, or hidden) already
/// decided what the file holds, so its raw output is never added.
static STEP_FAILURE_OUTPUTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Record the output of a step command that failed.
pub fn note_step_failure(output: &str) {
    let output = console::strip_ansi_codes(output);
    let output = output.trim();
    if !output.is_empty() {
        STEP_FAILURE_OUTPUTS.lock().unwrap().push(output.to_string());
    }
}

/// One step's section of the output file.
pub struct StepSummary {
    pub step: String,
    pub label: &'static str,
    pub status: &'static str,
    pub output: String,
}

/// Remove the output file left by an earlier run, so it never describes a run
/// other than the latest one.
pub fn clear() {
    let path = &*env::HK_OUTPUT_FILE;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!("Error removing output file {}: {e}", path.display()),
    }
}

/// Write every step's summary to the output file, without ANSI codes. Writes
/// nothing when no step produced output.
pub fn write_summaries(summaries: &[StepSummary]) {
    if summaries.is_empty() {
        return;
    }
    let mut content = String::new();
    for summary in summaries {
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&format!(
            "{} {} ({}):\n{}\n",
            summary.step,
            summary.label,
            summary.status,
            console::strip_ansi_codes(&summary.output).trim_end()
        ));
    }
    if let Err(e) = write(&content, false) {
        warn!("Error writing output file: {e:?}");
        return;
    }
    WRITTEN.store(true, Ordering::SeqCst);
}

/// Record the output of the command that failed the process, then point the
/// user at the output file. A step's failed command is already covered by the
/// hook's summaries; only a command outside any step, such as a stash command,
/// has its output written (appended when the hook wrote summaries).
pub fn write_failed_command(result: &ensembler::CmdResult) {
    let path = &*env::HK_OUTPUT_FILE;
    let output = console::strip_ansi_codes(&result.combined_output);
    let trimmed = output.trim();
    let from_step = !trimmed.is_empty()
        && STEP_FAILURE_OUTPUTS
            .lock()
            .unwrap()
            .iter()
            .any(|step_output| step_output.contains(trimmed));
    if from_step {
        // Point at the file only when it has summaries (a hidden step's
        // failure may be the only output).
        if WRITTEN.load(Ordering::SeqCst) {
            eprintln!("\nSee {} for full command output", path.display());
        }
        return;
    }
    let res = if WRITTEN.load(Ordering::SeqCst) {
        if trimmed.is_empty() {
            Ok(())
        } else {
            write(&format!("\nfailed command output:\n{output}"), true)
        }
    } else {
        write(&output, false)
    };
    if let Err(e) = res {
        warn!("Error writing output file: {e:?}");
        return;
    }
    eprintln!("\nSee {} for full command output", path.display());
}

fn write(content: &str, append: bool) -> std::io::Result<()> {
    let path: &Path = &env::HK_OUTPUT_FILE;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        env::create_state_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)?;
    file.write_all(content.as_bytes())
}
