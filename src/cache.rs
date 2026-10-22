use eyre::WrapErr;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::Result;
use itertools::Itertools;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::LazyLock as Lazy;

use crate::hash::hash_to_str;

#[derive(Debug, Clone)]
pub struct CacheManagerBuilder {
    cache_file_path: PathBuf,
    cache_keys: Vec<String>,
    fresh_duration: Option<Duration>,
    fresh_files: Vec<PathBuf>,
    fresh_file_key_mode: FreshFileKeyMode,
}

#[derive(Debug, Clone, Copy)]
enum FreshFileKeyMode {
    PathAndContent,
    ContentOnly,
}

pub static BASE_CACHE_KEYS: Lazy<Vec<String>> = Lazy::new(|| {
    [env!("CARGO_PKG_VERSION")]
        .into_iter()
        .map(|s| s.to_string())
        .collect()
});

impl CacheManagerBuilder {
    pub fn new(cache_file_path: impl AsRef<Path>) -> Self {
        let cache_file_path = cache_file_path.as_ref().to_path_buf();
        Self {
            cache_file_path,
            cache_keys: BASE_CACHE_KEYS.clone(),
            fresh_files: vec![],
            fresh_duration: None,
            fresh_file_key_mode: FreshFileKeyMode::PathAndContent,
        }
    }

    // pub fn with_fresh_duration(mut self, duration: Option<Duration>) -> Self {
    //     self.fresh_duration = duration;
    //     self
    // }

    pub fn with_fresh_files(mut self, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        self.fresh_files.extend(paths);
        self
    }

    pub fn with_cache_key(mut self, key: impl Into<String>) -> Self {
        self.cache_keys.push(key.into());
        self
    }

    pub fn with_content_fresh_files(mut self, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        self.fresh_files.extend(paths);
        self.fresh_file_key_mode = FreshFileKeyMode::ContentOnly;
        self
    }

    fn cache_key(&self) -> String {
        hash_to_str(&self.cache_keys)
    }

    /// Key on the fresh files' contents as they are now, so builders cloned
    /// from this one do not read them again.
    pub fn hash_fresh_files(mut self) -> Self {
        self.key_fresh_files();
        self
    }

    /// Draining the fresh files makes a second call a no-op.
    fn key_fresh_files(&mut self) {
        let mode = self.fresh_file_key_mode;
        self.cache_keys
            .extend(self.fresh_files.drain(..).unique().map(|path| match mode {
                FreshFileKeyMode::PathAndContent => fresh_file_cache_key(&path),
                FreshFileKeyMode::ContentOnly => fresh_file_content_cache_key(&path),
            }));
    }

    pub fn build<T>(mut self) -> CacheManager<T>
    where
        T: Serialize + DeserializeOwned,
    {
        self.key_fresh_files();
        let key = self.cache_key();
        let (base, ext) = split_file_name(&self.cache_file_path);
        let mut cache_file_path = self.cache_file_path;
        cache_file_path.set_file_name(format!("{base}-{key}.{ext}"));
        CacheManager {
            cache_file_path,
            cache: Box::new(OnceCell::new()),
            fresh_duration: self.fresh_duration,
        }
    }
}

fn split_file_name(path: &Path) -> (String, String) {
    let (base, ext) = path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .rsplit_once('.')
        .unwrap();
    (base.to_string(), ext.to_string())
}

#[derive(Debug, Clone)]
pub struct CacheManager<T>
where
    T: Serialize + DeserializeOwned,
{
    cache_file_path: PathBuf,
    fresh_duration: Option<Duration>,
    cache: Box<OnceCell<T>>,
}

impl<T> CacheManager<T>
where
    T: Serialize + DeserializeOwned,
{
    #[tracing::instrument(level = "info", name = "cache.get_or_try_init", skip_all, fields(path = %self.cache_file_path.display()))]
    pub fn get_or_try_init<F>(&self, fetch: F) -> Result<&T>
    where
        F: FnOnce() -> Result<T>,
    {
        let val = self.cache.get_or_try_init(|| {
            if let Some(val) = self.read_fresh() {
                return Ok::<_, eyre::Report>(val);
            }
            let path = &self.cache_file_path;
            let val = (fetch)()?;
            if let Err(err) = self.write(&val) {
                warn!("failed to write cache file: {} {:#}", path.display(), err);
            }
            Ok(val)
        })?;
        Ok(val)
    }

    /// Unlike `get_or_try_init`, a miss writes nothing, so the caller can
    /// choose the key to write under after computing the value.
    #[tracing::instrument(level = "info", name = "cache.get", skip_all, fields(path = %self.cache_file_path.display()))]
    pub fn get(&self) -> Option<T> {
        self.read_fresh()
    }

    fn read_fresh(&self) -> Option<T> {
        let path = &self.cache_file_path;
        if self.is_fresh() && *crate::env::HK_CACHE {
            match self.parse() {
                Ok(val) => {
                    tracing::event!(tracing::Level::INFO, "cache.hit");
                    return Some(val);
                }
                Err(err) => {
                    warn!("failed to parse cache file: {} {:#}", path.display(), err);
                }
            }
        }
        tracing::event!(tracing::Level::INFO, "cache.miss");
        None
    }

    fn parse(&self) -> Result<T> {
        let path = &self.cache_file_path;
        trace!("reading {}", path.display());
        // serde_json reads an unbuffered reader a byte per syscall.
        let bytes = std::fs::read(path)?;
        let val = serde_json::from_slice(&bytes)?;
        Ok(val)
    }

    /// hk processes that run at once (a pre-commit and a pre-push hook, agents
    /// working in parallel) share cache entries. An entry is written to a
    /// temporary file and renamed over the old one, so a reader sees all of
    /// the old entry or all of the new one, never a truncated file.
    pub fn write(&self, val: &T) -> Result<()> {
        static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);
        let path = &self.cache_file_path;
        tracing::info!(path = %path.display(), "cache.write");
        let bytes = serde_json::to_vec(val)?;
        if let Some(parent) = path.parent() {
            xx::file::create_dir_all(parent)?;
        }
        let tmp = path.with_extension(format!(
            "{}.{}.tmp",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        let written = std::fs::write(&tmp, bytes).and_then(|()| rename_over(&tmp, path));
        if written.is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
        written.wrap_err_with(|| format!("failed to write {}", path.display()))
    }

    #[cfg(test)]
    pub fn clear(&self) -> Result<()> {
        let path = &self.cache_file_path;
        trace!("clearing cache {}", path.display());
        if path.exists() {
            xx::file::remove_file(path)?;
        }
        Ok(())
    }

    fn is_fresh(&self) -> bool {
        if !self.cache_file_path.exists() {
            return false;
        }
        if let Some(fresh_duration) = self.fresh_duration
            && let Ok(cache_modified) = self.cache_file_path.metadata().and_then(|m| m.modified())
            && cache_modified.elapsed().unwrap_or_default() >= fresh_duration
        {
            return false;
        }
        true
    }
}

/// Renames `from` over `to`. Windows refuses to replace a file that a scanner
/// or indexer has open for a moment, so a denied rename is tried again, as
/// `stash_journal::replace` does.
fn rename_over(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempt = 0;
    loop {
        match std::fs::rename(from, to) {
            Err(err)
                if cfg!(windows)
                    && err.kind() == std::io::ErrorKind::PermissionDenied
                    && attempt < 20 =>
            {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(25));
            }
            result => return result,
        }
    }
}

fn fresh_file_cache_key(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(contents) => format!("{}:{}", path.display(), hash_to_str(&contents)),
        Err(_) => format!("{}:<missing>", path.display()),
    }
}

fn fresh_file_content_cache_key(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(contents) => hash_to_str(&contents),
        Err(_) => "<missing>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::env;

    use super::*;

    #[test]
    fn test_cache() {
        let cache = CacheManagerBuilder::new(env::HK_CACHE_DIR.join("test-cache.json")).build();
        cache.clear().unwrap();
        let val = cache.get_or_try_init(|| Ok(1)).unwrap();
        assert_eq!(val, &1);
        let val = cache.get_or_try_init(|| Ok(2)).unwrap();
        assert_eq!(val, &1);
    }

    #[test]
    fn get_does_not_write_on_a_miss() {
        let cache =
            CacheManagerBuilder::new(env::HK_CACHE_DIR.join("get-miss-test.json")).build::<u8>();
        cache.clear().unwrap();
        assert_eq!(cache.get(), None);
        assert!(!cache.cache_file_path.exists());
    }

    #[test]
    fn get_reads_an_entry_only_when_caching_is_enabled() {
        let cache =
            CacheManagerBuilder::new(env::HK_CACHE_DIR.join("get-hit-test.json")).build::<u8>();
        cache.write(&1).unwrap();
        assert_eq!(cache.get(), env::HK_CACHE.then_some(1));
    }

    #[test]
    fn explicit_cache_keys_select_distinct_files() {
        let base = env::HK_CACHE_DIR.join("cache-key-test.json");
        let unset = CacheManagerBuilder::new(&base)
            .with_cache_key("HK_PKL_HTTP_REWRITE=<unset>")
            .build::<u8>();
        let rewrite = CacheManagerBuilder::new(&base)
            .with_cache_key("HK_PKL_HTTP_REWRITE=https://example.com/=https://mirror.example.com/")
            .build::<u8>();

        assert_ne!(unset.cache_file_path, rewrite.cache_file_path);
    }

    #[test]
    fn cache_keys_sharing_a_hash_prefix_select_distinct_files() {
        let base = env::HK_CACHE_DIR.join("cache-key-prefix-test.json");
        let builder = |key: &str| CacheManagerBuilder::new(&base).with_cache_key(key);
        let mut seen = std::collections::HashMap::new();
        let (a, b) = (0..)
            .map(|i| format!("key-{i}"))
            .find_map(|key| {
                let prefix: String = builder(&key).cache_key().chars().take(5).collect();
                seen.insert(prefix, key.clone()).map(|other| (other, key))
            })
            .unwrap();

        assert_ne!(
            builder(&a).build::<u8>().cache_file_path,
            builder(&b).build::<u8>().cache_file_path
        );
    }

    fn entry_in(dir: &Path) -> CacheManager<Vec<u32>> {
        CacheManagerBuilder::new(dir.join("entry.json")).build()
    }

    fn file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// A write that truncated the entry and wrote it again would change what a
    /// reader that already opened the entry sees, partway through; only a
    /// rename leaves the old file whole. Windows cannot replace an open file.
    #[cfg(unix)]
    #[test]
    fn write_replaces_the_entry_instead_of_rewriting_it() {
        use std::io::Read;
        let dir = tempfile::tempdir().unwrap();
        let cache = entry_in(dir.path());
        cache.write(&vec![1; 100_000]).unwrap();
        let mut reader = std::fs::File::open(&cache.cache_file_path).unwrap();

        cache.write(&vec![2; 100_000]).unwrap();

        let mut old = Vec::new();
        reader.read_to_end(&mut old).unwrap();
        assert_eq!(
            serde_json::from_slice::<Vec<u32>>(&old).unwrap(),
            vec![1; 100_000]
        );
        assert_eq!(cache.parse().unwrap(), vec![2; 100_000]);
    }

    #[test]
    fn write_leaves_only_the_entry_behind() {
        let dir = tempfile::tempdir().unwrap();
        let cache = entry_in(dir.path());
        cache.write(&vec![1]).unwrap();
        cache.write(&vec![2]).unwrap();
        assert_eq!(
            file_names(dir.path()),
            [cache
                .cache_file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()]
        );
    }

    #[test]
    fn a_failed_write_removes_its_temporary_file_and_names_the_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cache = entry_in(dir.path());
        // A directory cannot be replaced by a file, so the rename fails.
        std::fs::create_dir(&cache.cache_file_path).unwrap();

        let err = cache.write(&vec![1]).unwrap_err();

        assert!(format!("{err:#}").contains(&cache.cache_file_path.display().to_string()));
        assert_eq!(
            file_names(dir.path()),
            [cache
                .cache_file_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()]
        );
    }
}
