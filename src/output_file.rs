//! The output file (`HK_OUTPUT_FILE`, `output.log` by default) holds the step
//! summaries of the latest hook run, so a reader without the console (a coding
//! agent, a CI artifact) sees every step's output, not only the first failure.
//! The output directory (`HK_OUTPUT_DIR`, `output/` by default) holds the same
//! summaries with one `<step>.log` file per step, so a reader that wants one
//! step's output reads only that step.

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

/// Whether this process wrote per-step files to the output directory.
static WROTE_STEP_FILES: AtomicBool = AtomicBool::new(false);

/// Output of every failed step command. A command that ends the process with
/// one of these belongs to a step, whose summary (rendered, or hidden) already
/// decided what the file holds, so its raw output is never added.
static STEP_FAILURE_OUTPUTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Record the output of a step command that failed.
pub fn note_step_failure(output: &str) {
    let output = console::strip_ansi_codes(output);
    let output = output.trim();
    if !output.is_empty() {
        STEP_FAILURE_OUTPUTS
            .lock()
            .unwrap()
            .push(output.to_string());
    }
}

/// One step's section of the output file.
pub struct StepSummary {
    pub step: String,
    pub label: &'static str,
    pub status: &'static str,
    pub output: String,
}

/// Remove the output file and per-step files left by an earlier run, so they
/// never describe a run other than the latest one.
pub fn clear() {
    let path = &*env::HK_OUTPUT_FILE;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!("Error removing output file {}: {e}", path.display()),
    }
    clear_step_files(&env::HK_OUTPUT_DIR);
}

/// Remove the `.log` files directly inside the output directory, then the
/// directory itself when that leaves it empty. Other files and subdirectories
/// are left alone, since the directory can be set to a path hk does not own.
fn clear_step_files(dir: &Path) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            warn!("Error reading output directory {}: {e}", dir.display());
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_log = path.extension().is_some_and(|ext| ext == "log");
        if !is_log || !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        if let Err(e) = std::fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!("Error removing output file {}: {e}", path.display());
        }
    }
    // Fails, as intended, when the directory still holds other entries.
    let _ = std::fs::remove_dir(dir);
}

/// The file name of a step's file in the output directory. Characters other
/// than ASCII letters, digits, `-`, `_` and `.` are percent-encoded, so every
/// step name maps to its own file and none can name a path outside the
/// directory.
pub fn step_file_name(step: &str) -> String {
    let mut name = String::with_capacity(step.len() + 4);
    for byte in step.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => name.push(byte as char),
            _ => name.push_str(&format!("%{byte:02X}")),
        }
    }
    name.push_str(".log");
    name
}

/// Write every step's summary to the output file, without ANSI codes. Writes
/// nothing when no step produced output.
pub fn write_summaries(summaries: &[StepSummary]) {
    if summaries.is_empty() {
        return;
    }
    let mut content = String::new();
    let dir = &*env::HK_OUTPUT_DIR;
    let mut wrote_step_file = false;
    for summary in summaries {
        let section = format!(
            "{} {} ({}):\n{}\n",
            summary.step,
            summary.label,
            summary.status,
            console::strip_ansi_codes(&summary.output).trim_end()
        );
        let step_path = dir.join(step_file_name(&summary.step));
        match write_file(&step_path, &section) {
            Ok(()) => wrote_step_file = true,
            Err(e) => warn!("Error writing output file {}: {e:?}", step_path.display()),
        }
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&section);
    }
    WROTE_STEP_FILES.store(wrote_step_file, Ordering::SeqCst);
    if let Err(e) = write(&content, false) {
        warn!("Error writing output file: {e:?}");
        return;
    }
    WRITTEN.store(true, Ordering::SeqCst);
}

/// Point the user at the output file, and at the per-step files when the hook
/// wrote them.
fn print_hint(path: &Path) {
    if WROTE_STEP_FILES.load(Ordering::SeqCst) {
        eprintln!(
            "\nSee {} for full command output, or {} for one file per step",
            path.display(),
            env::HK_OUTPUT_DIR.display()
        );
    } else {
        eprintln!("\nSee {} for full command output", path.display());
    }
}

/// Record the output of the command that failed the process, then point the
/// user at the output file. A step's failed command is already covered by the
/// hook's summaries; only a command outside any step, such as a stash command,
/// has its output written (appended when the hook wrote summaries).
pub fn write_failed_command(result: &ensembler::CmdResult) {
    let path = &*env::HK_OUTPUT_FILE;
    let output = console::strip_ansi_codes(&result.combined_output);
    let trimmed = output.trim();
    // Output already covered by a step's summary, or no output at all, adds
    // nothing; point at the file only when it has summaries.
    let nothing_to_add = trimmed.is_empty()
        || STEP_FAILURE_OUTPUTS
            .lock()
            .unwrap()
            .iter()
            .any(|step_output| step_output.contains(trimmed));
    if nothing_to_add {
        if WRITTEN.load(Ordering::SeqCst) {
            print_hint(path);
        }
        return;
    }
    let res = if WRITTEN.load(Ordering::SeqCst) {
        write(&format!("\nfailed command output:\n{output}"), true)
    } else {
        write(&output, false)
    };
    if let Err(e) = res {
        warn!("Error writing output file: {e:?}");
        return;
    }
    print_hint(path);
}

/// Replace the output file with `content` (or the current file followed by
/// `content` when appending). The text goes to a temporary file beside it that
/// is then renamed into place, so a reader never sees a partial file.
fn write(content: &str, append: bool) -> std::io::Result<()> {
    let path: &Path = &env::HK_OUTPUT_FILE;
    let mut full = if append {
        match std::fs::read_to_string(path) {
            Ok(existing) => existing,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(e),
        }
    } else {
        String::new()
    };
    full.push_str(content);
    write_file(path, &full)
}

/// Replace `path` with `content`, creating its directory. The text goes to a
/// temporary file beside it that is then renamed into place.
fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
    let dir = match path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        Some(parent) => {
            env::create_state_dir_all(parent)?;
            parent
        }
        None => Path::new("."),
    };
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(content.as_bytes())?;
    // On Windows a scanner or reader can hold the old file open for a moment;
    // retry the replace like the cache does. A failed rename drops the
    // temporary file.
    let temp = temp.into_temp_path();
    crate::cache::rename_over(&temp, path)?;
    // The temporary path is gone now; nothing is left to clean up.
    let _ = temp.keep();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::step_file_name;

    #[test]
    fn step_file_names_are_safe_and_distinct() {
        assert_eq!(step_file_name("cargo-fmt"), "cargo-fmt.log");
        assert_eq!(step_file_name("py.lint_2"), "py.lint_2.log");
        assert_eq!(step_file_name("a/b"), "a%2Fb.log");
        assert_eq!(step_file_name("a b"), "a%20b.log");
        assert_eq!(step_file_name("a%2Fb"), "a%252Fb.log");
        assert_eq!(step_file_name(".."), "...log");
        assert_eq!(step_file_name("é"), "%C3%A9.log");
    }
}
