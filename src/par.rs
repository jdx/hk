//! Per-file filesystem checks, spread over a few threads and shared between
//! steps.
//!
//! A step over every file in the repository stats or reads thousands of
//! files before its command starts. Each check is a separate system call
//! whose cost is mostly the path lookup, so they run several times faster in
//! parallel, and steps that check the same files can share the results.

use dashmap::DashMap;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

/// Fewest items worth a thread of their own. Starting a thread costs about
/// as much as a few dozen `stat` calls.
const MIN_ITEMS_PER_THREAD: usize = 256;
/// Filesystem checks stop speeding up well before this many threads.
const MAX_THREADS: usize = 8;

static THREADS: LazyLock<usize> = LazyLock::new(|| {
    std::thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1)
        .min(MAX_THREADS)
});

/// Apply `f` to every item, returning the results in the items' order.
///
/// Small inputs run on the calling thread. Large ones are split into
/// contiguous chunks on scoped threads; a chunk whose thread cannot be
/// started runs on the calling thread instead.
pub fn map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = (*THREADS).min(items.len() / MIN_ITEMS_PER_THREAD);
    if threads <= 1 {
        return items.iter().map(&f).collect();
    }
    let f = &f;
    let chunk_len = items.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let mut chunks = items.chunks(chunk_len);
        // The calling thread takes the first chunk itself.
        let first = chunks.next().unwrap_or_default();
        let handles = chunks
            .map(|chunk| {
                std::thread::Builder::new()
                    .spawn_scoped(scope, move || chunk.iter().map(f).collect::<Vec<_>>())
                    .map_err(|_| chunk)
            })
            .collect::<Vec<_>>();
        let mut results = Vec::with_capacity(items.len());
        results.extend(first.iter().map(f));
        for handle in handles {
            match handle {
                Ok(handle) => match handle.join() {
                    Ok(chunk_results) => results.extend(chunk_results),
                    Err(panic) => std::panic::resume_unwind(panic),
                },
                Err(chunk) => results.extend(chunk.iter().map(f)),
            }
        }
        results
    })
}

/// Keep the items for which `keep` returns true, in order, evaluating
/// `keep` in parallel as [`map`] does.
pub fn retain<T: Sync>(items: &mut Vec<T>, keep: impl Fn(&T) -> bool + Sync) {
    let mut keep = map(items, keep).into_iter();
    items.retain(|_| keep.next().unwrap_or(true));
}

/// A per-path cache that computes each value once, even when several steps
/// ask for the same file at the same time.
///
/// Steps filter their files concurrently when a hook starts, and most of them
/// ask about the same files. A step that finds another step already reading
/// a file waits for that result instead of reading the file again.
pub struct PathMemo<V> {
    slots: DashMap<PathBuf, Arc<Slot<V>>>,
}

struct Slot<V> {
    value: OnceLock<V>,
    init: Mutex<()>,
}

impl<V> Default for Slot<V> {
    fn default() -> Self {
        Self {
            value: OnceLock::new(),
            init: Mutex::new(()),
        }
    }
}

impl<V: Clone> PathMemo<V> {
    pub fn new() -> Self {
        Self {
            slots: DashMap::new(),
        }
    }

    /// The cached value for `path`, computing it with `init` if no value is
    /// cached. A `None` from `init` is returned but not cached, so a later
    /// call tries again.
    pub fn get_or_try_init(&self, path: &Path, init: impl FnOnce() -> Option<V>) -> Option<V> {
        let slot = match self.slots.get(path) {
            Some(slot) => match slot.value.get() {
                Some(value) => return Some(value.clone()),
                None => slot.clone(),
            },
            None => self.slots.entry(path.to_path_buf()).or_default().clone(),
        };
        let _init = slot.init.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(value) = slot.value.get() {
            return Some(value.clone());
        }
        let value = init()?;
        let _ = slot.value.set(value.clone());
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memo_computes_once_and_retries_failures() {
        let memo = PathMemo::new();
        let path = Path::new("a");
        assert_eq!(memo.get_or_try_init(path, || None::<u32>), None);
        assert_eq!(memo.get_or_try_init(path, || Some(1)), Some(1));
        assert_eq!(memo.get_or_try_init(path, || Some(2)), Some(1));
    }

    #[test]
    fn memo_shares_concurrent_computations() {
        let memo = PathMemo::new();
        let calls = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..8 {
                s.spawn(|| {
                    let value = memo.get_or_try_init(Path::new("a"), || {
                        calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        Some(7)
                    });
                    assert_eq!(value, Some(7));
                });
            }
        });
        assert_eq!(calls.into_inner(), 1);
    }

    #[test]
    fn map_preserves_order_for_large_inputs() {
        let items = (0..MIN_ITEMS_PER_THREAD * 10).collect::<Vec<_>>();
        let doubled = map(&items, |i| i * 2);
        assert_eq!(doubled, items.iter().map(|i| i * 2).collect::<Vec<_>>());
    }

    #[test]
    fn retain_keeps_matching_items_in_order() {
        let mut items = (0..MIN_ITEMS_PER_THREAD * 10).collect::<Vec<_>>();
        let expected = items
            .iter()
            .copied()
            .filter(|i| i % 3 == 0)
            .collect::<Vec<_>>();
        retain(&mut items, |i| i % 3 == 0);
        assert_eq!(items, expected);

        let mut small = vec![1, 2, 3];
        retain(&mut small, |i| *i != 2);
        assert_eq!(small, vec![1, 3]);
    }
}
