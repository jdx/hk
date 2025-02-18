use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{
    env,
    git::Git,
    step::{RunType, Step},
    step_scheduler::StepScheduler,
    version, Result,
};
use miette::{bail, Context, IntoDiagnostic};

impl Config {
    pub fn get() -> Result<Self> {
        let paths = if let Some(file) = env::HK_FILE.as_ref() {
            vec![file.as_str()]
        } else {
            vec!["hk.pkl", "hk.toml", "hk.yaml", "hk.yml", "hk.json"]
        };
        let mut cwd = std::env::current_dir().into_diagnostic()?;
        while cwd != Path::new("/") {
            for path in &paths {
                let path = cwd.join(path);
                if path.exists() {
                    return Self::read(&path).wrap_err_with(|| {
                        format!("Failed to read config file: {}", path.display())
                    });
                }
            }
            cwd = cwd.parent().map(PathBuf::from).unwrap_or_default();
        }
        debug!("No config file found, using default");
        Ok(Self::default())
    }

    pub fn read(path: &Path) -> Result<Self> {
        let ext = path.extension().unwrap_or_default().to_str().unwrap();
        let mut config: Config = match ext {
            "toml" => {
                let raw = xx::file::read_to_string(path)?;
                toml::from_str(&raw).into_diagnostic()?
            }
            "yaml" => {
                let raw = xx::file::read_to_string(path)?;
                serde_yaml::from_str(&raw).into_diagnostic()?
            }
            "json" => {
                let raw = xx::file::read_to_string(path)?;
                serde_json::from_str(&raw).into_diagnostic()?
            }
            "pkl" => {
                match rpkl::from_config(path) {
                    Ok(config) => config,
                    Err(err) => {
                        // if pkl bin is not installed
                        if which::which("pkl").is_err() {
                            bail!("install pkl cli to use pkl config files https://pkl-lang.org/");
                        } else {
                            return Err(err).into_diagnostic()?;
                        }
                    }
                }
            }
            _ => {
                bail!("Unsupported file extension: {}", ext);
            }
        };
        if let Some(min_hk_version) = &config.min_hk_version {
            version::version_cmp_or_bail(min_hk_version)?;
        }
        if let Some(pre_commit) = &mut config.pre_commit {
            for (name, step) in pre_commit.iter_mut() {
                step.name = name.clone();
            }
        }
        Ok(config)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub min_hk_version: Option<String>,
    #[serde(rename = "pre-commit")]
    pub pre_commit: Option<IndexMap<String, Step>>,
    #[serde(rename = "pre-push")]
    pub pre_push: Option<IndexMap<String, Step>>,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(self).unwrap())
    }
}

impl Config {
    pub async fn run_hook(
        &self,
        hook: &IndexMap<String, Step>,
        run_type: RunType,
        repo: &Git,
    ) -> Result<()> {
        let files = if matches!(run_type, RunType::CheckAll | RunType::FixAll) {
            repo.all_files()?
        } else {
            repo.staged_files()?
        };
        StepScheduler::new(hook, run_type, repo)
            .with_files(files)
            .run()
            .await
    }
}
