use clx::progress::{ProgressJobBuilder, ProgressStatus};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};
use tokio::sync::Mutex;
use tokio::{signal, sync::OnceCell};

use crate::{
    Result,
    cache::CacheManagerBuilder,
    env,
    git::Git,
    hash,
    step::{CheckType, LinterStep, RunType},
    step_scheduler::StepScheduler,
    tera::Context,
    ui::style,
    version,
};
use eyre::{WrapErr, bail};

impl Config {
    pub fn get() -> Result<Self> {
        let paths = if let Some(file) = env::HK_FILE.as_ref() {
            vec![file.as_str()]
        } else {
            vec!["hk.pkl", "hk.toml", "hk.yaml", "hk.yml", "hk.json"]
        };
        let mut cwd = std::env::current_dir()?;
        while cwd != Path::new("/") {
            for path in &paths {
                let path = cwd.join(path);
                if path.exists() {
                    let hash_key = format!("{}.json", hash::hash_to_str(&path));
                    let hash_key_path = env::HK_CACHE_DIR.join("configs").join(hash_key);
                    return CacheManagerBuilder::new(hash_key_path)
                        .with_fresh_file(path.to_path_buf())
                        .build()
                        .get_or_try_init(|| {
                            Self::read(&path).wrap_err_with(|| {
                                format!("Failed to read config file: {}", path.display())
                            })
                        })
                        .cloned();
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
                toml::from_str(&raw)?
            }
            "yaml" => {
                let raw = xx::file::read_to_string(path)?;
                serde_yaml::from_str(&raw)?
            }
            "json" => {
                let raw = xx::file::read_to_string(path)?;
                serde_json::from_str(&raw)?
            }
            "pkl" => {
                match rpkl::from_config(path) {
                    Ok(config) => config,
                    Err(err) => {
                        // if pkl bin is not installed
                        if which::which("pkl").is_err() {
                            bail!("install pkl cli to use pkl config files https://pkl-lang.org/");
                        } else {
                            return Err(err).wrap_err("failed to read pkl config file");
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
        for (name, hook) in config.hooks.iter_mut() {
            hook.init(name);
        }
        Ok(config)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub min_hk_version: Option<String>,
    #[serde(default)]
    pub hooks: IndexMap<String, Hook>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Hook {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub steps: IndexMap<String, LinterStep>,
    #[serde(default)]
    pub fix: bool,
    pub stash: Option<StashMethod>,
}

impl Hook {
    fn init(&mut self, hook_name: &str) {
        self.name = hook_name.to_string();
        for (name, step) in self.steps.iter_mut() {
            step.init(name);
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize, strum::EnumString)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum StashMethod {
    Git,
    PatchFile,
    None,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(self).unwrap())
    }
}

impl Config {
    pub async fn run_hook(
        &self,
        all: bool,
        hook: &str,
        linters: &[String],
        tctx: Context,
        from_ref: Option<&str>,
        to_ref: Option<&str>,
    ) -> Result<()> {
        if env::HK_SKIP_HOOK.contains(hook) {
            warn!("{}: skipping hook due to HK_SKIP_HOOK", hook);
            return Ok(());
        }
        static HOOK: LazyLock<Hook> = LazyLock::new(Default::default);
        let hook = self.hooks.get(hook).unwrap_or(&HOOK);

        let mut hk_progress = ProgressJobBuilder::new()
            .body(vec!["{{hk}}{{hook}}{{message}}".to_string()])
            .prop(
                "hk",
                &format!(
                    "{} {} {}",
                    style::emagenta("hk").bold(),
                    style::edim(version::version()),
                    style::edim("by @jdx")
                )
                .to_string(),
            );

        if hook.name == "check" || hook.name == "fix" {
            hk_progress = hk_progress.prop("hook", "");
        } else {
            hk_progress = hk_progress.prop(
                "hook",
                &style::edim(format!(" – {}", hook.name)).to_string(),
            );
        }

        let run_type = if *env::HK_FIX && hook.fix {
            hk_progress = hk_progress.prop("message", &style::edim(" – fix").to_string());
            RunType::Fix
        } else {
            hk_progress = hk_progress.prop("message", &style::edim(" – check").to_string());
            RunType::Check(CheckType::Check)
        };
        // Check if both from_ref and to_ref are provided or neither
        if from_ref.is_some() != to_ref.is_some() {
            return Err(eyre::eyre!(
                "Both --from-ref and --to-ref must be provided together"
            ));
        }
        let hk_progress = hk_progress.start();
        let repo = Arc::new(Mutex::new(Git::new()?));

        let file_progress = ProgressJobBuilder::new().body(vec![
            "{{spinner()}} files - {{message}}{% if files is defined %} ({{files}} file{{files|pluralize}}){% endif %}".to_string(),
        ])
        .prop("message", "Fetching git status")
        .start();
        // TODO: this doesn't necessarily need to be fetched right now, or at least blocking
        let git_status = OnceCell::new();
        let files = if let (Some(from), Some(to)) = (from_ref, to_ref) {
            file_progress.prop(
                "message",
                &format!("Fetching files between {} and {}", from, to),
            );
            repo.lock().await.files_between_refs(from, to)?
        } else if all {
            file_progress.prop("message", "Fetching all files in repo");
            repo.lock().await.all_files()?
        } else if hook.name == "check" || hook.name == "fix" {
            // TODO: this should probably be customizable on the hook like `fix = true` is
            file_progress.prop("message", "Fetching modified files");
            repo.lock().await.modified_files()?
        } else {
            file_progress.prop("message", "Fetching staged files");
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            git_status.staged_files.iter().cloned().collect()
        };
        file_progress.prop("files", &files.len());
        file_progress.set_status(ProgressStatus::Done);

        watch_for_ctrl_c(repo.clone());

        let stash_method = env::HK_STASH
            .as_ref()
            .or(hook.stash.as_ref())
            .unwrap_or(&StashMethod::None);
        if stash_method != &StashMethod::None {
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            repo.lock()
                .await
                .stash_unstaged(&file_progress, stash_method, git_status)?;
        }

        let mut result = StepScheduler::new(hook, run_type, repo.clone())
            .with_files(files)
            .with_linters(linters)
            .with_tctx(tctx)
            .run()
            .await;
        hk_progress.set_status(ProgressStatus::Done);

        if let Err(err) = repo.lock().await.pop_stash() {
            if result.is_ok() {
                result = Err(err);
            } else {
                warn!("Failed to pop stash: {}", err);
            }
        }
        result
    }
}

fn watch_for_ctrl_c(repo: Arc<Mutex<Git>>) {
    tokio::spawn(async move {
        if let Err(err) = signal::ctrl_c().await {
            warn!("Failed to watch for ctrl-c: {}", err);
        }
        if let Err(err) = repo.lock().await.pop_stash() {
            warn!("Failed to pop stash: {}", err);
        }
        clx::progress::flush();
        // TODO: gracefully stop child processes
        std::process::exit(1);
    });
}
