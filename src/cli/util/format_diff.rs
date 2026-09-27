use crate::Result;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Semaphore;

/// Print a patch of what a formatter would change, for a `check_diff` command
///
/// Runs COMMAND once for each file, with the file on stdin, and compares what
/// it prints with the file. `{}` in COMMAND is replaced with the file's path,
/// for formatters that take the path of their input as an option. With
/// `--no-stdin`, COMMAND reads the file itself, from the path `{}` gives it.
/// Files are formatted in parallel.
///
/// Prints a unified diff for every file that would change and exits 1, or
/// exits 0 when none would. If COMMAND fails for any file, or prints nothing
/// for a file that isn't empty, no patch is printed: its error output is
/// shown and hk runs the step's fixer instead.
///
/// Example: `hk util format-diff a.lua b.lua -- stylua --stdin-filepath {} -`
#[derive(Debug, usage_rs::Args)]
pub struct FormatDiff {
    /// Give COMMAND no stdin, for a formatter that reads the file at `{}`
    #[usage(long)]
    pub no_stdin: bool,

    /// Files to format
    #[usage(arg, required)]
    pub files: Vec<PathBuf>,

    /// The formatter, reading stdin and writing the formatted file to stdout
    #[usage(arg, required, double_dash = "required")]
    pub command: Vec<String>,
}

enum Outcome {
    Unchanged,
    Changed(String),
    Failed { code: i32, message: String },
}

impl FormatDiff {
    pub async fn run(&self) -> Result<()> {
        let jobs = std::thread::available_parallelism().map_or(4, |n| n.get());
        let semaphore = Arc::new(Semaphore::new(jobs));
        let mut handles = Vec::with_capacity(self.files.len());
        for file in &self.files {
            let semaphore = semaphore.clone();
            let file = file.clone();
            let command = self.command.clone();
            let stdin = !self.no_stdin;
            handles.push(tokio::spawn(async move {
                let _permit = semaphore.acquire_owned().await;
                format_file(&file, &command, stdin).await
            }));
        }

        let mut patches = String::new();
        let mut failures = Vec::new();
        for handle in handles {
            match handle.await? {
                Outcome::Unchanged => {}
                Outcome::Changed(patch) => patches.push_str(&patch),
                Outcome::Failed { code, message } => failures.push((code, message)),
            }
        }

        // A patch for only the files that formatted would let hk apply it and
        // report success, hiding the failure. Without one, hk runs the fixer,
        // which reports it.
        if let Some(&(code, _)) = failures.first() {
            let mut err = io::stderr().lock();
            for (_, message) in &failures {
                err.write_all(message.as_bytes())?;
                if !message.ends_with('\n') {
                    err.write_all(b"\n")?;
                }
            }
            std::process::exit(code);
        }
        if !patches.is_empty() {
            let mut out = io::stdout().lock();
            out.write_all(patches.as_bytes())?;
            out.flush()?;
            std::process::exit(1);
        }
        Ok(())
    }
}

async fn format_file(file: &PathBuf, command: &[String], stdin: bool) -> Outcome {
    let path = file.to_string_lossy();
    let failed = |message: String| Outcome::Failed { code: 1, message };
    let original = match std::fs::read(file) {
        Ok(bytes) => bytes,
        Err(err) => return failed(format!("{path}: {err}")),
    };
    let Ok(original) = String::from_utf8(original) else {
        return failed(format!("{path}: not valid UTF-8"));
    };

    let (program, args) = command.split_first().expect("command is required");
    let mut child = match Command::new(program)
        .args(args.iter().map(|arg| arg.replace("{}", &path)))
        .stdin(if stdin { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => return failed(format!("{program}: {err}")),
    };
    // Write stdin while the output is read, so a large file can't fill both
    // pipes and deadlock.
    let writer = child.stdin.take().map(|mut pipe| {
        let input = original.clone();
        tokio::spawn(async move {
            pipe.write_all(input.as_bytes()).await?;
            // Close stdin so the formatter sees the end of its input.
            drop(pipe);
            std::io::Result::Ok(())
        })
    });
    let output = match child.wait_with_output().await {
        Ok(output) => output,
        Err(err) => return failed(format!("{program}: {err}")),
    };
    // A formatter that stopped reading early may have formatted only part of
    // the file, even if it exits 0.
    if let Some(writer) = writer {
        match writer.await {
            Ok(Ok(())) => {}
            Ok(Err(err)) => return failed(format!("{path}: writing to {program}: {err}")),
            Err(err) => return failed(format!("{path}: writing to {program}: {err}")),
        }
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Outcome::Failed {
            code: output.status.code().filter(|&c| c != 0).unwrap_or(1),
            message: format!("{path}: {program} failed\n{stderr}"),
        };
    }
    let Ok(formatted) = String::from_utf8(output.stdout) else {
        return failed(format!("{path}: {program} printed invalid UTF-8"));
    };
    // Diffing against empty output would make a patch that empties the file.
    if formatted.is_empty() && !original.is_empty() {
        return failed(format!("{path}: {program} printed nothing\n{stderr}"));
    }
    if formatted == original {
        return Outcome::Unchanged;
    }
    Outcome::Changed(crate::diff::render_unified_diff(
        &original, &formatted, &path, &path,
    ))
}
