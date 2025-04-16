use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::Mutex;
use tokio::{signal, sync::OnceCell};

use crate::{
    Result,
    cache::CacheManagerBuilder,
    env,
    git::Git,
    glob, hash,
    hook_options::HookOptions,
    step::{CheckType, LinterStep, RunType},
    step_scheduler::StepScheduler,
    ui::style,
    version,
};
use eyre::{WrapErr, bail};

impl Config {
    pub fn get() -> Result<Self> {
        let default_path = env::HK_FILE
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("hk.pkl");
        let paths = vec![default_path, "hk.toml", "hk.yaml", "hk.yml", "hk.json"];
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
        let mut config = Config::default();
        config.init(Path::new(default_path))?;
        Ok(config)
    }

    fn read(path: &Path) -> Result<Self> {
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
                match parse_pkl("pkl", path) {
                    Ok(raw) => raw,
                    Err(err) => {
                        // if pkl bin is not installed
                        if which::which("pkl").is_err() {
                            if let Ok(out) = parse_pkl("mise x -- pkl", path) {
                                return Ok(out);
                            };
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
        config.init(path)?;
        Ok(config)
    }

    fn init(&mut self, path: &Path) -> Result<()> {
        self.path = path.to_path_buf();
        if let Some(min_hk_version) = &self.min_hk_version {
            version::version_cmp_or_bail(min_hk_version)?;
        }
        for (name, hook) in self.hooks.iter_mut() {
            hook.init(name);
        }
        Ok(())
    }
}

fn parse_pkl<T: DeserializeOwned>(bin: &str, path: &Path) -> Result<T> {
    let json = xx::process::sh(&format!("{bin} eval -f json {}", path.display()))?;
    serde_json::from_str(&json).wrap_err("failed to parse pkl config file")
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct Config {
    pub min_hk_version: Option<String>,
    #[serde(default)]
    pub hooks: IndexMap<String, Hook>,
    #[serde(skip)]
    #[serde(default)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
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

    pub async fn run(&self, opts: HookOptions) -> Result<()> {
        if env::HK_SKIP_HOOK.contains(&self.name) {
            warn!("{}: skipping hook due to HK_SKIP_HOOK", &self.name);
            return Ok(());
        }
        let run_type = if *env::HK_FIX && self.fix {
            RunType::Fix
        } else {
            RunType::Check(CheckType::Check)
        };
        let hk_progress = self.start_hk_progress(run_type);
        if opts.to_ref.is_some() {
            // TODO: implement to_ref
        }
        let repo = Arc::new(Mutex::new(Git::new()?));

        let file_progress = ProgressJobBuilder::new().body(vec![
            "{{spinner()}} files - {{message}}{% if files is defined %} ({{files}} file{{files|pluralize}}){% endif %}".to_string(),
        ])
        .prop("message", "Fetching git status")
        .start();
        // TODO: this doesn't necessarily need to be fetched right now, or at least blocking
        let git_status = OnceCell::new();
        let stash_method = env::HK_STASH.or(self.stash).unwrap_or(StashMethod::None);
        let mut files = if let Some(files) = &opts.files {
            files
                .iter()
                .map(|f| {
                    let p = PathBuf::from(f);
                    if p.is_dir() {
                        all_files_in_dir(&p)
                    } else {
                        Ok(vec![p])
                    }
                })
                .flatten_ok()
                .collect::<Result<BTreeSet<_>>>()?
        } else if let Some(glob) = &opts.glob {
            file_progress.prop("message", "Fetching files matching glob");
            // TODO: should fetch just the files that match the glob
            let all_files = repo.lock().await.all_files()?;
            glob::get_matches(glob, &all_files)?.into_iter().collect()
        } else if let (Some(from), Some(to)) = (&opts.from_ref, &opts.to_ref) {
            file_progress.prop(
                "message",
                &format!("Fetching files between {} and {}", from, to),
            );
            repo.lock()
                .await
                .files_between_refs(from, to)?
                .into_iter()
                .collect()
        } else if opts.all {
            file_progress.prop("message", "Fetching all files in repo");
            repo.lock().await.all_files()?.into_iter().collect()
        } else if stash_method != StashMethod::None {
            file_progress.prop("message", "Fetching staged files");
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            git_status.staged_files.iter().cloned().collect()
        } else {
            file_progress.prop("message", "Fetching modified files");
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            git_status
                .staged_files
                .iter()
                .chain(git_status.unstaged_files.iter())
                .cloned()
                .collect()
        };
        for exclude in opts.exclude.unwrap_or_default() {
            let exclude = Path::new(&exclude);
            files.retain(|f| !f.starts_with(exclude));
        }
        if let Some(exclude_glob) = &opts.exclude_glob {
            let f = files.iter().collect::<Vec<_>>();
            let exclude_files = glob::get_matches(exclude_glob, &f)?
                .into_iter()
                .collect::<HashSet<_>>();
            files.retain(|f| !exclude_files.contains(f));
        }
        file_progress.prop("files", &files.len());
        file_progress.set_status(ProgressStatus::Done);

        watch_for_ctrl_c(repo.clone());

        if stash_method != StashMethod::None {
            let git_status = git_status
                .get_or_try_init(async || repo.lock().await.status())
                .await?;
            repo.lock()
                .await
                .stash_unstaged(&file_progress, stash_method, git_status)?;
        }

        let mut result = StepScheduler::new(self, run_type, repo.clone())
            .with_files(files.into_iter().collect())
            .with_linters(&opts.step)
            .with_tctx(opts.tctx)
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

    fn start_hk_progress(&self, run_type: RunType) -> Arc<ProgressJob> {
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
        if self.name == "check" || self.name == "fix" {
            hk_progress = hk_progress.prop("hook", "");
        } else {
            hk_progress = hk_progress.prop(
                "hook",
                &style::edim(format!(" – {}", self.name)).to_string(),
            );
        }
        if run_type == RunType::Fix {
            hk_progress = hk_progress.prop("message", &style::edim(" – fix").to_string());
        } else {
            hk_progress = hk_progress.prop("message", &style::edim(" – check").to_string());
        }
        hk_progress.start()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize, strum::EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
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

impl Config {
    pub fn validate(&self) -> Result<()> {
        // TODO: validate config
        Ok(())
    }
}

fn all_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = vec![];
    for entry in xx::file::ls(dir)? {
        if entry.is_dir() {
            files.extend(all_files_in_dir(&entry)?);
        } else {
            files.push(entry);
        }
    }
    Ok(files)
}
