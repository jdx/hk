use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use crate::Result;

/// Cache of `mise env` results, resolved at most once per directory per hk
/// invocation. Keyed by the step's `dir` (relative to the repo root).
static CACHE: LazyLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::OnceCell<EnvMap>>>>> =
    LazyLock::new(Default::default);

type EnvMap = Arc<IndexMap<String, String>>;

/// Resolve the mise environment for a directory by running `mise env --json`
/// there. This picks up tools and env vars defined by that directory's mise
/// config (e.g. a subproject's mise.toml in a monorepo), which the hk process
/// itself doesn't have when it was started from the repo root.
///
/// Returns an empty map when mise is unavailable or fails; the error is logged
/// once per directory.
pub async fn mise_env_for_dir(dir: &Path) -> EnvMap {
    let cell = CACHE
        .lock()
        .unwrap()
        .entry(dir.to_path_buf())
        .or_default()
        .clone();
    cell.get_or_init(|| async {
        match fetch(dir).await {
            Ok(env) => Arc::new(env),
            Err(err) => {
                warn!("mise env failed in {}: {err}", dir.display());
                Default::default()
            }
        }
    })
    .await
    .clone()
}

async fn fetch(dir: &Path) -> Result<IndexMap<String, String>> {
    let output = tokio::process::Command::new("mise")
        .args(["env", "--json"])
        .current_dir(dir)
        .stdin(std::process::Stdio::null())
        .output()
        .await?;
    if !output.status.success() {
        eyre::bail!(
            "mise env exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}
