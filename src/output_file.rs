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

/// Output of failed commands from steps with `output_summary = "hide"`, which
/// must stay out of the file even when such a command ends the process.
static HIDDEN_OUTPUTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Record a failed command's output from a step whose summary is hidden.
pub fn note_hidden_output(output: &str) {
    let output = console::strip_ansi_codes(output);
    let output = output.trim();
    if !output.is_empty() {
        HIDDEN_OUTPUTS.lock().unwrap().push(output.to_string());
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
/// user at the output file. When the hook already wrote its summaries, the
/// command's output is appended only if no summary already holds it (a command
/// outside any step, such as a stash or report command).
pub fn write_failed_command(result: &ensembler::CmdResult) {
    let path = &*env::HK_OUTPUT_FILE;
    let output = console::strip_ansi_codes(&result.combined_output);
    let trimmed = output.trim();
    let hidden = !trimmed.is_empty()
        && HIDDEN_OUTPUTS
            .lock()
            .unwrap()
            .iter()
            .any(|hidden| hidden.contains(trimmed));
    if hidden {
        // The step opted out of its summary; point at the file only when other
        // steps' summaries are in it.
        if WRITTEN.load(Ordering::SeqCst) {
            eprintln!("\nSee {} for full command output", path.display());
        }
        return;
    }
    let res = if WRITTEN.load(Ordering::SeqCst) {
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        if trimmed.is_empty() || existing.contains(trimmed) {
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
