use crate::env;
use crate::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct Plugin {
    pub url: Option<String>,
    pub path: Option<PathBuf>,
}

impl From<String> for Plugin {
    fn from(s: String) -> Self {
        let mut plugin = Plugin::default();
        if s.starts_with("http") {
            plugin.url = Some(s);
        } else if s.starts_with("/") || s.starts_with(".") || s.starts_with("~") {
            plugin.path = Some(PathBuf::from(s));
        } else {
            panic!("Invalid plugin: {}", s);
        }
        plugin
    }
}

impl Plugin {
    pub async fn run(&self) -> Result<()> {
        if let Some(path) = &self.path {
            let wasm = extism::Wasm::file(path);
            return super::extism::run(wasm);
        }
        if let Some(url) = &self.url {
            let hashed_url = xx::hash::hash_to_str(url);
            let cache_path = env::ANGLER_CACHE_DIR.join("plugins").join(&hashed_url[..8]);
            if !cache_path.exists() {
                xx::http::download(url, &cache_path).await?;
            }
            let wasm = extism::Wasm::file(&cache_path);
            return super::extism::run(wasm);
        }
        Ok(())
    }
}
