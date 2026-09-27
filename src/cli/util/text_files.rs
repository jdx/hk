//! File handling shared by the text fixers (`trailing-whitespace` and
//! `end-of-file-fixer`).
//!
//! Both run over every text file in a repository, so they read each file with
//! one `stat` and one `open`, spread the files over the available CPUs, and
//! buffer their output. Results are still reported in argument order, and a
//! failure stops the run at the same file as a sequential loop would.

use crate::Result;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, mpsc};

/// How many leading bytes decide whether a file is text.
const TEXT_PROBE_BYTES: u64 = 8192;

/// Fewest files worth handing to another thread; below this, starting the
/// thread costs more than the files take to read.
const MIN_FILES_PER_THREAD: usize = 64;

/// How many files the workers may read ahead of the results already handled.
const RESULTS_AHEAD: usize = 256;

/// The size of `path` if it is a regular file, following symlinks. Missing
/// paths, directories, and special files are not text files and are skipped.
pub(super) fn regular_file_len(path: &Path) -> Option<u64> {
    let metadata = fs::metadata(path).ok()?;
    metadata.is_file().then_some(metadata.len())
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

/// Run `f` on every file across the available CPUs and hand each result to
/// `emit` on the calling thread, in file order, so output and writes happen
/// exactly as in a sequential loop. Workers stay at most [`RESULTS_AHEAD`]
/// files ahead of `emit`, which bounds how many results are held at once.
/// The first error from `emit` stops the run and is returned; no later
/// result reaches `emit`.
pub(super) fn for_each_in_order<T, F, E>(files: &[PathBuf], f: F, mut emit: E) -> Result<()>
where
    T: Send,
    F: Fn(&Path) -> T + Sync,
    E: FnMut(&Path, T) -> Result<()>,
{
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(files.len() / MIN_FILES_PER_THREAD)
        .max(1);
    if threads == 1 {
        for path in files {
            emit(path, f(path))?;
        }
        return Ok(());
    }

    struct Progress {
        /// The next file a worker will claim.
        claimed: usize,
        /// How many files `emit` has handled.
        emitted: usize,
        stop: bool,
    }
    let progress = Mutex::new(Progress {
        claimed: 0,
        emitted: 0,
        stop: false,
    });
    let room = Condvar::new();
    let (tx, rx) = mpsc::channel::<(usize, T)>();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            let (tx, progress, room, f) = (tx.clone(), &progress, &room, &f);
            scope.spawn(move || {
                loop {
                    let i = {
                        let mut p = progress.lock().unwrap();
                        while !p.stop
                            && p.claimed < files.len()
                            && p.claimed >= p.emitted + RESULTS_AHEAD
                        {
                            p = room.wait(p).unwrap();
                        }
                        if p.stop || p.claimed >= files.len() {
                            return;
                        }
                        p.claimed += 1;
                        p.claimed - 1
                    };
                    if tx.send((i, f(&files[i]))).is_err() {
                        return;
                    }
                }
            });
        }
        drop(tx);

        let mut pending = BTreeMap::new();
        let mut next = 0;
        let mut emitted = || -> Result<()> {
            for (i, result) in &rx {
                pending.insert(i, result);
                while let Some(result) = pending.remove(&next) {
                    emit(&files[next], result)?;
                    next += 1;
                    progress.lock().unwrap().emitted = next;
                    room.notify_all();
                }
            }
            Ok(())
        };
        let result = emitted();
        if result.is_err() {
            progress.lock().unwrap().stop = true;
            room.notify_all();
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(n: usize) -> Vec<PathBuf> {
        (0..n).map(|i| PathBuf::from(i.to_string())).collect()
    }

    fn index(path: &Path) -> usize {
        path.to_str().unwrap().parse().unwrap()
    }

    #[test]
    fn emits_every_result_in_file_order() {
        let files = files(5000);
        let mut seen = Vec::new();
        for_each_in_order(&files, index, |path, i| {
            assert_eq!(index(path), i);
            seen.push(i);
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, (0..5000).collect::<Vec<_>>());
    }

    #[test]
    fn stops_at_the_first_error() {
        let files = files(5000);
        let mut seen = Vec::new();
        let result = for_each_in_order(&files, index, |_, i| {
            if i == 700 {
                eyre::bail!("failed at {i}");
            }
            seen.push(i);
            Ok(())
        });
        assert_eq!(result.unwrap_err().to_string(), "failed at 700");
        assert_eq!(seen, (0..700).collect::<Vec<_>>());
    }
}
