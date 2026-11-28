//! The output file (`HK_OUTPUT_FILE`, `output.log` by default) holds the step
//! summaries of the latest hook run, so a reader without the console (a coding
//! agent, a CI artifact) sees every step's output, not only the first failure.
//! The output directory (`HK_OUTPUT_DIR`, `output/` by default) holds the same
//! summaries with one `<step>.log` file per step, so a reader that wants one
//! step's output reads only that step.

use crate::env;
use std::{
    collections::HashSet,
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
    let _lock = lock();
    let path = &*env::HK_OUTPUT_FILE;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!("Error removing output file {}: {e}", path.display()),
    }
    let dir = &*env::HK_OUTPUT_DIR;
    remove_step_files(dir, &HashSet::new());
    // Fails, as intended, when the directory still holds other entries.
    let _ = std::fs::remove_dir(dir);
}

/// Hold the output lock, so hook runs that overlap clear and write the output
/// file and directory one at a time: the run that writes last leaves exactly
/// its own step files, never a mix of two runs. Without the lock (it could not
/// be opened) the run goes ahead unlocked.
fn lock() -> Option<std::fs::File> {
    let path = &*env::HK_OUTPUT_LOCK;
    let file = path
        .parent()
        .map_or(Ok(()), env::create_state_dir_all)
        .and_then(|()| {
            std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(path)
        })
        .and_then(|file| file.lock().map(|()| file));
    match file {
        Ok(file) => Some(file),
        Err(e) => {
            debug!("Error locking output lock {}: {e}", path.display());
            None
        }
    }
}

/// Remove the `.log` files directly inside the output directory, except those
/// named in `keep`. Other files and subdirectories are left alone, since the
/// directory can be set to a path hk does not own.
fn remove_step_files(dir: &Path, keep: &HashSet<String>) {
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
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| keep.contains(name))
        {
            continue;
        }
        if let Err(e) = std::fs::remove_file(&path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!("Error removing output file {}: {e}", path.display());
        }
    }
}

/// The longest a step file's name gets before `.log`, well under the 255-byte
/// limit of common filesystems.
const MAX_STEM_LEN: usize = 200;

/// The file name of a step's file in the output directory. Characters other
/// than ASCII letters, digits, `-`, `_` and `.` are percent-encoded, so every
/// step name maps to its own file and none can name a path outside the
/// directory. A name longer than [`MAX_STEM_LEN`] is cut short and ends in `~`
/// and a hash of the step name; encoding never writes `~`, so a shortened name
/// cannot match another step's full one.
pub fn step_file_name(step: &str) -> String {
    let mut name = String::with_capacity(step.len() + 4);
    for byte in step.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' => name.push(byte as char),
            _ => name.push_str(&format!("%{byte:02X}")),
        }
    }
    if name.len() > MAX_STEM_LEN {
        // Room for `~` and a 16-digit hash; cut before a `%XX` escape rather
        // than through it.
        let mut cut = MAX_STEM_LEN - 17;
        if let Some(pct) = name[cut.saturating_sub(2)..cut].rfind('%') {
            cut = cut - 2 + pct;
        }
        name.truncate(cut);
        name.push_str(&format!("~{:0>16}", crate::hash::hash_to_str(&step)));
    }
    name.push_str(".log");
    name
}

/// Write every step's summary to the output file, and each one to its own file
/// in the output directory, without ANSI codes. Writes nothing when no step
/// produced output.
pub fn write_summaries(summaries: &[StepSummary]) {
    if summaries.is_empty() {
        return;
    }
    let mut content = String::new();
    let mut step_files = Vec::with_capacity(summaries.len());
    for summary in summaries {
        let section = format!(
            "{} {} ({}):\n{}\n",
            summary.step,
            summary.label,
            summary.status,
            console::strip_ansi_codes(&summary.output).trim_end()
        );
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&section);
        step_files.push((step_file_name(&summary.step), section));
    }

    let _lock = lock();
    let dir = &*env::HK_OUTPUT_DIR;
    // A run that overlapped this one may have written files for steps this run
    // has no output for; they would describe the other run.
    let keep = step_files.iter().map(|(name, _)| name.clone()).collect();
    remove_step_files(dir, &keep);
    let mut wrote_step_file = false;
    for (name, section) in &step_files {
        let step_path = dir.join(name);
        match write_file(&step_path, section) {
            Ok(()) => wrote_step_file = true,
            Err(e) => warn!("Error writing output file {}: {e:?}", step_path.display()),
        }
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
    let _lock = lock();
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
    use super::*;

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

    #[test]
    fn long_step_file_names_are_shortened_and_distinct() {
        let a = step_file_name(&"é".repeat(60));
        let b = step_file_name(&format!("{}x", "é".repeat(60)));
        assert!(a.len() <= MAX_STEM_LEN + 4, "{a}");
        assert!(b.len() <= MAX_STEM_LEN + 4, "{b}");
        assert_ne!(a, b);
        assert!(a.ends_with(".log"));
        // The cut never splits an escape.
        let stem = a.split('~').next().unwrap();
        assert!(stem.len() % 3 == 0 && stem.starts_with("%C3%A9"), "{stem}");
        // A name at the limit keeps its full form.
        let exact = "a".repeat(MAX_STEM_LEN);
        assert_eq!(step_file_name(&exact), format!("{exact}.log"));
    }

    #[test]
    fn removing_step_files_keeps_the_named_ones_and_other_files() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["kept.log", "stale.log", "notes.txt"] {
            std::fs::write(dir.path().join(name), "x").unwrap();
        }
        let keep = HashSet::from(["kept.log".to_string()]);
        remove_step_files(dir.path(), &keep);
        assert!(dir.path().join("kept.log").exists());
        assert!(!dir.path().join("stale.log").exists());
        assert!(dir.path().join("notes.txt").exists());
    }
}
