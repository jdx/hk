use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};

use crate::{
    Result,
    cache::CacheManagerBuilder,
    env,
    git::Git,
    hash,
    step::{CheckType, RunType, Step},
    step_scheduler::StepScheduler,
    tera::Context,
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
            hook.name = name.clone();
            for (name, step) in hook.steps.iter_mut() {
                match step {
                    Steps::Step(step) => step.name = name.clone(),
                    Steps::Stash(_) => {}
                }
            }
        }
        for (name, linter) in config.linters.iter_mut() {
            linter.name = name.clone();
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
    pub linters: IndexMap<String, Linter>,
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
    pub steps: IndexMap<String, Steps>,
    #[serde(default)]
    pub fix: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Steps {
    Step(Box<Step>),
    Stash(Box<Stash>),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Stash {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub method: StashMethod,
}

#[derive(Debug, Clone, Eq, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum StashMethod {
    #[default]
    GitDiff,
    GitStash,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
#[serde_as]
pub struct Linter {
    #[serde(default)]
    pub name: String,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub glob: Option<Vec<String>>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub profiles: Option<Vec<String>>,
    pub exclusive: bool,
    pub check_first: bool,
    pub batch: bool,
    pub stomp: bool,
    pub check: Option<String>,
    pub check_diff: Option<String>,
    pub check_list_files: Option<String>,
    pub check_extra_args: Option<String>,
    pub fix: Option<String>,
    pub fix_extra_args: Option<String>,
    pub workspace_indicator: Option<String>,
    pub prefix: Option<String>,
    pub dir: Option<String>,
    pub env: IndexMap<String, String>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub stage: Option<Vec<String>>,
    pub depends: Vec<String>,
    #[serde(default)]
    pub linter_dependencies: IndexMap<String, Vec<String>>,
}

impl From<Linter> for Step {
    fn from(linter: Linter) -> Self {
        Step {
            r#type: Some("linter".to_string()),
            glob: linter.glob,
            profiles: linter.profiles,
            exclusive: linter.exclusive,
            check_first: linter.check_first,
            batch: linter.batch,
            stomp: linter.stomp,
            check: linter.check,
            check_diff: linter.check_diff,
            check_list_files: linter.check_list_files,
            check_extra_args: linter.check_extra_args,
            fix: linter.fix,
            fix_extra_args: linter.fix_extra_args,
            workspace_indicator: linter.workspace_indicator,
            prefix: linter.prefix,
            dir: linter.dir,
            env: linter.env,
            stage: linter.stage,
            name: linter.name,
            depends: linter.depends,
            run: None,
            root: None,
            linter_dependencies: linter.linter_dependencies,
        }
    }
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
        let run_type = if *env::HK_FIX && hook.fix {
            RunType::Fix
        } else {
            RunType::Check(CheckType::Check)
        };
        // Check if both from_ref and to_ref are provided or neither
        if from_ref.is_some() != to_ref.is_some() {
            return Err(eyre::eyre!(
                "Both --from-ref and --to-ref must be provided together"
            ));
        }

        let mut repo = Git::new()?;
        if !all {
            repo.stash_unstaged(false)?;
        }
        let file_progress_builder = ProgressJobBuilder::new().body(vec![
            "{{spinner()}} files - {{message}}{% if files is defined %} ({{files}} file{{files|pluralize}}){% endif %}".to_string(),
        ]);
        let file_progress: Arc<ProgressJob>;
        let files = if let (Some(from), Some(to)) = (from_ref, to_ref) {
            file_progress = file_progress_builder
                .prop(
                    "message",
                    &format!("Fetching files between {} and {}", from, to),
                )
                .start();
            repo.files_between_refs(from, to)?
        } else if all {
            file_progress = file_progress_builder
                .prop("message", "Fetching all files in repo")
                .start();
            repo.all_files()?
        } else {
            file_progress = file_progress_builder
                .prop("message", "Fetching staged files")
                .start();
            repo.staged_files()?
        };
        file_progress.prop("files", &files.len());
        file_progress.set_status(ProgressStatus::Done);

        let mut result = StepScheduler::new(&hook, run_type, &repo)
            .with_files(files)
            .with_linters(linters)
            .with_tctx(tctx)
            .run()
            .await;

        if let Err(err) = repo.pop_stash() {
            if result.is_ok() {
                result = Err(err);
            } else {
                warn!("Failed to pop stash: {}", err);
            }
        }
        result
    }
}
