//! Checks that passed during a fix run, and the fixers that changed their
//! files afterwards.
//!
//! Steps run in parallel. File locks keep a fixer from writing a file while a
//! check reads it, but they don't order the two: a check can pass on a file
//! that a fixer rewrites a moment later, and nothing would check the new
//! contents. So a check-only job that passes in a fix run is recorded here.
//! A job that takes write locks on one of its files watches that file, and if
//! the contents change before the locks are released, the check is stale. The
//! hook runs stale checks again once every step has finished.

use crate::step_job::StepJob;
use indexmap::IndexSet;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct PassedChecks {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    checks: Vec<PassedCheck>,
    /// The checks that read each file, by position in `checks`.
    by_file: HashMap<PathBuf, Vec<usize>>,
}

struct PassedCheck {
    job: StepJob,
    /// Steps that changed one of the job's files after it passed.
    changed_by: IndexSet<String>,
}

/// A check that passed before a fixer changed one of its files.
pub struct StaleCheck {
    pub job: StepJob,
    pub changed_by: IndexSet<String>,
}

/// The contents of a writer's files that passed checks have read, taken when
/// it locked them. Dropped before the locks are released, it compares the
/// files again and marks the checks of any that changed stale.
pub struct WriteWatch {
    checks: Arc<PassedChecks>,
    step: String,
    before: Vec<(PathBuf, Option<u64>)>,
}

impl std::fmt::Debug for WriteWatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WriteWatch({}, {} files)", self.step, self.before.len())
    }
}

impl PassedChecks {
    /// Remember that `job` passed. Call while the job still holds its file
    /// locks, so no writer can change its files before it is recorded.
    pub fn record(&self, job: &StepJob) {
        let mut job = job.clone();
        job.progress = None;
        let mut inner = self.inner.lock().unwrap();
        let index = inner.checks.len();
        for file in &job.files {
            inner.by_file.entry(file.clone()).or_default().push(index);
        }
        inner.checks.push(PassedCheck {
            job,
            changed_by: IndexSet::new(),
        });
    }

    /// Start watching the files among `files` that a passed check has read.
    /// `step` is about to hold write locks on `files`.
    pub fn watch(self: &Arc<Self>, step: &str, files: &[PathBuf]) -> Option<WriteWatch> {
        let watched: Vec<PathBuf> = {
            let inner = self.inner.lock().unwrap();
            if inner.by_file.is_empty() {
                return None;
            }
            files
                .iter()
                .filter(|file| inner.by_file.contains_key(*file))
                .cloned()
                .collect()
        };
        if watched.is_empty() {
            return None;
        }
        let before = watched
            .into_iter()
            .map(|file| {
                let hash = content_hash(&file);
                (file, hash)
            })
            .collect();
        Some(WriteWatch {
            checks: self.clone(),
            step: step.to_string(),
            before,
        })
    }

    /// The checks a later writer made stale, removed from this record.
    pub fn take_stale(&self) -> Vec<StaleCheck> {
        let mut inner = self.inner.lock().unwrap();
        inner.by_file.clear();
        std::mem::take(&mut inner.checks)
            .into_iter()
            .filter(|check| !check.changed_by.is_empty())
            .map(|check| StaleCheck {
                job: check.job,
                changed_by: check.changed_by,
            })
            .collect()
    }
}

impl Drop for WriteWatch {
    fn drop(&mut self) {
        let changed: Vec<&PathBuf> = self
            .before
            .iter()
            .filter(|(file, before)| content_hash(file) != *before)
            .map(|(file, _)| file)
            .collect();
        if changed.is_empty() {
            return;
        }
        let mut inner = self.checks.inner.lock().unwrap();
        let Inner { checks, by_file } = &mut *inner;
        for file in changed {
            for &index in by_file.get(file).into_iter().flatten() {
                let check = &mut checks[index];
                if check.job.step.name != self.step {
                    debug!(
                        "{}: {} changed {} after the check passed",
                        check.job.step.name,
                        self.step,
                        file.display()
                    );
                    check.changed_by.insert(self.step.clone());
                }
            }
        }
    }
}

/// A hash of `path`'s contents, or `None` if it can't be read (for example,
/// because it was deleted).
fn content_hash(path: &Path) -> Option<u64> {
    let content = std::fs::read(path).ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut hasher);
    Some(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::{RunType, Step};

    fn check_job(name: &str, files: &[&Path]) -> StepJob {
        let step = Step {
            name: name.to_string(),
            ..Default::default()
        };
        StepJob::new(
            Arc::new(step),
            files.iter().map(|f| f.to_path_buf()).collect(),
            RunType::Fix,
        )
    }

    #[test]
    fn a_write_that_changes_a_checked_file_makes_its_check_stale() {
        let dir = tempfile::tempdir().unwrap();
        let changed = dir.path().join("changed.txt");
        let untouched = dir.path().join("untouched.txt");
        std::fs::write(&changed, "a").unwrap();
        std::fs::write(&untouched, "a").unwrap();
        let checks = Arc::new(PassedChecks::default());
        checks.record(&check_job("lint", &[&changed]));
        checks.record(&check_job("other", &[&untouched]));

        let watch = checks.watch("fmt", &[changed.clone(), untouched.clone()]);
        std::fs::write(&changed, "b").unwrap();
        drop(watch);

        let stale = checks.take_stale();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].job.step.name, "lint");
        assert_eq!(stale[0].changed_by.iter().collect::<Vec<_>>(), vec!["fmt"]);
        assert!(checks.take_stale().is_empty());
    }

    #[test]
    fn a_write_that_leaves_the_contents_alone_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "a").unwrap();
        let checks = Arc::new(PassedChecks::default());
        checks.record(&check_job("lint", &[&file]));

        let watch = checks.watch("fmt", std::slice::from_ref(&file));
        std::fs::write(&file, "a").unwrap();
        drop(watch);

        assert!(checks.take_stale().is_empty());
    }

    #[test]
    fn files_no_check_read_are_not_watched() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "a").unwrap();
        let checks = Arc::new(PassedChecks::default());
        assert!(checks.watch("fmt", std::slice::from_ref(&file)).is_none());
        checks.record(&check_job("lint", &[&dir.path().join("other.txt")]));
        assert!(checks.watch("fmt", std::slice::from_ref(&file)).is_none());
    }

    #[test]
    fn a_step_does_not_make_its_own_check_stale() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "a").unwrap();
        let checks = Arc::new(PassedChecks::default());
        checks.record(&check_job("lint", &[&file]));

        let watch = checks.watch("lint", std::slice::from_ref(&file));
        std::fs::write(&file, "b").unwrap();
        drop(watch);

        assert!(checks.take_stale().is_empty());
    }
}
