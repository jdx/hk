use once_cell::sync::OnceCell;
use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::Result;
use itertools::Itertools;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::LazyLock as Lazy;

use crate::hash::hash_to_str;

#[derive(Debug)]
pub struct CacheManagerBuilder {
    cache_file_path: PathBuf,
    cache_keys: Vec<String>,
    fresh_duration: Option<Duration>,
    fresh_files: Vec<PathBuf>,
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
        }
    }

    // pub fn with_fresh_duration(mut self, duration: Option<Duration>) -> Self {
    //     self.fresh_duration = duration;
    //     self
    // }

    pub fn with_fresh_file(mut self, path: PathBuf) -> Self {
        self.fresh_files.push(path);
        self
    }

    // pub fn with_cache_key(mut self, key: String) -> Self {
    //     self.cache_keys.push(key);
    //     self
    // }

    fn cache_key(&self) -> String {
        hash_to_str(&self.cache_keys).chars().take(5).collect()
    }

    pub fn build<T>(self) -> CacheManager<T>
    where
        T: Serialize + DeserializeOwned,
    {
        let key = self.cache_key();
        let (base, ext) = split_file_name(&self.cache_file_path);
        let mut cache_file_path = self.cache_file_path;
        cache_file_path.set_file_name(format!("{base}-{key}.{ext}"));
        CacheManager {
            cache_file_path,
            cache: Box::new(OnceCell::new()),
            fresh_files: self.fresh_files,
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
    fresh_files: Vec<PathBuf>,
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
            let path = &self.cache_file_path;
            if self.is_fresh() && !cfg!(debug_assertions) {
                match self.parse() {
                    Ok(val) => {
                        tracing::event!(tracing::Level::INFO, "cache.hit");
                        return Ok::<_, eyre::Report>(val);
                    }
                    Err(err) => {
                        warn!("failed to parse cache file: {} {:#}", path.display(), err);
                    }
                }
            }
            tracing::event!(tracing::Level::INFO, "cache.miss");
            let val = (fetch)()?;
            if let Err(err) = self.write(&val) {
                warn!("failed to write cache file: {} {:#}", path.display(), err);
            }
            Ok(val)
        })?;
        Ok(val)
    }

    fn parse(&self) -> Result<T> {
        let path = &self.cache_file_path;
        trace!("reading {}", path.display());
        let mut f = File::open(path)?;
        let val = serde_json::from_reader(&mut f)?;
        Ok(val)
    }

    pub fn write(&self, val: &T) -> Result<()> {
        trace!("writing {}", self.cache_file_path.display());
        if let Some(parent) = self.cache_file_path.parent() {
            xx::file::create_dir_all(parent)?;
        }
        let mut f = File::create(&self.cache_file_path)?;
        f.write_all(&serde_json::to_vec(val)?)?;
        Ok(())
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
        if let Some(fresh_duration) = self.freshest_duration() {
            if let Ok(metadata) = self.cache_file_path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    return modified.elapsed().unwrap_or_default() < fresh_duration;
                }
            }
        }
        true
    }

    fn freshest_duration(&self) -> Option<Duration> {
        let mut freshest = self.fresh_duration;
        for path in self.fresh_files.iter().unique() {
            let duration = modified_duration(path).unwrap_or_default();
            freshest = Some(match freshest {
                None => duration,
                Some(freshest) => min(freshest, duration),
            })
        }
        freshest
    }
}

fn modified_duration(path: &Path) -> Option<Duration> {
    let metadata = path.metadata().ok()?;
    let modified = metadata.modified().ok()?;
    Some(modified.elapsed().unwrap_or_default())
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
}
