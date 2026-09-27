//! File handling shared by the text fixers (`trailing-whitespace` and
//! `end-of-file-fixer`).
//!
//! Both run over every text file in a repository, so they read each file with
//! one `stat` and one `open`, spread the files over the available CPUs, and
//! buffer their output. Results are still reported in argument order, and a
//! failure stops the run at the same file as a sequential loop would.

use crate::Result;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many leading bytes decide whether a file is text.
const TEXT_PROBE_BYTES: u64 = 8192;

/// Fewest files worth handing to another thread; below this, starting the
/// thread costs more than the files take to read.
const MIN_FILES_PER_THREAD: usize = 64;

/// Files are claimed in runs of this many, so threads rarely contend on the
/// shared counter but still even out files of very different sizes.
const CLAIM: usize = 16;

/// The size of `path` if it is a regular file, following symlinks. Missing
/// paths, directories, and special files are not text files and are skipped.
pub(super) fn regular_file_len(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_file().then(|| metadata.len())
}

/// Read the bytes that decide whether a file of `len` bytes is text: the first
/// 8 KiB, which must contain no NUL byte and be valid UTF-8 on their own.
/// Returns `None` for a binary file.
pub(super) fn read_text_probe(file: &mut File, len: u64) -> Result<Option<Vec<u8>>> {
    let mut head = vec![0; TEXT_PROBE_BYTES.min(len) as usize];
    file.read_exact(&mut head)?;
    if head.contains(&0) || std::str::from_utf8(&head).is_err() {
        return Ok(None);
    }
    Ok(Some(head))
}

/// Read the rest of `file` after `bytes` and decode it all, failing as
/// `fs::read_to_string` does when the content is not UTF-8.
pub(super) fn read_rest_to_string(file: &mut File, mut bytes: Vec<u8>) -> Result<String> {
    file.read_to_end(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
        .into()
    })
}

/// Run `f` on every item, in parallel when there are enough of them, and
/// return the results in the order of `items`.
pub(super) fn par_map<I, T, F>(items: &[I], f: F) -> Vec<T>
where
    I: Sync,
    T: Send,
    F: Fn(&I) -> T + Sync,
{
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(items.len() / MIN_FILES_PER_THREAD)
        .max(1);
    if threads == 1 {
        return items.iter().map(&f).collect();
    }
    let next = AtomicUsize::new(0);
    let done: Vec<Vec<(usize, T)>> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let start = next.fetch_add(CLAIM, Ordering::Relaxed);
                        if start >= items.len() {
                            break done;
                        }
                        let end = (start + CLAIM).min(items.len());
                        for (i, item) in items[start..end].iter().enumerate() {
                            done.push((start + i, f(item)));
                        }
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| match worker.join() {
                Ok(done) => done,
                Err(panic) => std::panic::resume_unwind(panic),
            })
            .collect()
    });
    let mut results: Vec<Option<T>> = std::iter::repeat_with(|| None).take(items.len()).collect();
    for (i, result) in done.into_iter().flatten() {
        results[i] = Some(result);
    }
    results
        .into_iter()
        .map(|result| result.expect("every item has a result"))
        .collect()
}

/// Write the fixed content of each file that has any. Like a sequential
/// fixer, nothing after the first file that failed to read is written, and
/// that error is returned; otherwise the first write error in file order is.
pub(super) fn write_fixes(files: &[PathBuf], fixes: Vec<Result<Option<String>>>) -> Result<()> {
    let mut writes = Vec::new();
    let mut read_error = None;
    for (path, fix) in files.iter().zip(fixes) {
        match fix {
            Ok(Some(content)) => writes.push((path, content)),
            Ok(None) => {}
            Err(err) => {
                read_error = Some(err);
                break;
            }
        }
    }
    for written in par_map(&writes, |(path, content)| fs::write(path, content)) {
        written?;
    }
    match read_error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}
