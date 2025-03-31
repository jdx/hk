use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressJobDoneBehavior, ProgressStatus};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
};
use tokio::sync::Mutex;

use crate::{
    Result,
    cache::CacheManagerBuilder,
    env,
    git::Git,
    hash,
    step::{CheckType, LinterStep, RunStep, RunType, exec_step},
    step_context::StepContext,
    step_job::StepJob,
    step_response::StepResponse,
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
            hook.name = name.clone();
            for (name, step) in hook.steps.iter_mut() {
                match step {
                    Steps::Run(step) => step.name = name.clone(),
                    Steps::Linter(step) => step.name = name.clone(),
                    Steps::Stash(step) => step.name = name.clone(),
                }
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
    Run(Box<RunStep>),
    Linter(Box<LinterStep>),
    Stash(Box<Stash>),
}

impl Steps {
    pub fn name(&self) -> &str {
        match self {
            Steps::Linter(step) => &step.name,
            Steps::Run(step) => &step.name,
            Steps::Stash(step) => &step.name,
        }
    }

    pub fn dir(&self) -> Option<&str> {
        match self {
            Steps::Linter(step) => step.dir.as_deref(),
            Steps::Run(step) => step.dir.as_deref(),
            Steps::Stash(_) => None,
        }
    }

    pub fn env(&self) -> &IndexMap<String, String> {
        static EMPTY: LazyLock<IndexMap<String, String>> = LazyLock::new(Default::default);
        match self {
            Steps::Linter(step) => &step.env,
            Steps::Run(step) => &step.env,
            Steps::Stash(_) => &EMPTY,
        }
    }

    pub fn glob(&self) -> Option<&Vec<String>> {
        match self {
            Steps::Linter(step) => step.glob.as_ref(),
            Steps::Run(step) => step.glob.as_ref(),
            Steps::Stash(_) => None,
        }
    }

    pub fn prefix(&self) -> Option<&str> {
        match self {
            Steps::Linter(step) => step.prefix.as_deref(),
            Steps::Run(_step) => None,
            Steps::Stash(_) => None,
        }
    }

    pub fn stage(&self) -> Option<&Vec<String>> {
        match self {
            Steps::Linter(step) => step.stage.as_ref(),
            Steps::Run(step) => step.stage.as_ref(),
            Steps::Stash(_) => None,
        }
    }

    pub fn run_cmd(&self, run_type: RunType) -> Option<&str> {
        match self {
            Steps::Linter(step) => step.run_cmd(run_type),
            Steps::Run(step) => Some(&step.run),
            Steps::Stash(_) => None,
        }
    }

    pub fn stomp(&self) -> bool {
        match self {
            Steps::Linter(step) => step.stomp,
            _ => false,
        }
    }

    pub fn depends(&self) -> &Vec<String> {
        static EMPTY: Vec<String> = vec![];
        match self {
            Steps::Linter(step) => &step.depends,
            Steps::Run(step) => &step.depends,
            _ => &EMPTY,
        }
    }

    pub fn available_run_type(&self, run_type: RunType) -> Option<RunType> {
        match self {
            Steps::Linter(step) => step.available_run_type(run_type),
            _ => Some(run_type),
        }
    }

    pub fn is_profile_enabled(&self) -> bool {
        match self {
            Steps::Linter(step) => step.is_profile_enabled(),
            Steps::Run(step) => step.is_profile_enabled(),
            _ => true,
        }
    }

    pub async fn run(&self, ctx: &StepContext, job: &StepJob) -> Result<StepResponse> {
        match self {
            Steps::Linter(_step) => exec_step(self, ctx, job).await,
            Steps::Run(_step) => exec_step(self, ctx, job).await,
            Steps::Stash(step) => step.run(ctx, job).await,
        }
    }

    pub(crate) fn build_step_progress(&self) -> Arc<ProgressJob> {
        ProgressJobBuilder::new()
            .body(vec![
                "{{spinner()}} {{name}} {% if message %}– {{message | flex}}{% endif %}"
                    .to_string(),
            ])
            .body_text(Some(vec![
                "{% if message %}{{spinner()}} {{name}} – {{message}}{% endif %}".to_string(),
            ]))
            .prop("name", self.name())
            .status(ProgressStatus::RunningCustom(style::edim("❯").to_string()))
            .on_done(if *env::HK_HIDE_WHEN_DONE {
                ProgressJobDoneBehavior::Hide
            } else {
                ProgressJobDoneBehavior::Keep
            })
            .start()
    }
}

impl fmt::Display for Steps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Steps::Linter(step) => write!(f, "{}", step),
            Steps::Run(step) => write!(f, "{}", step),
            Steps::Stash(step) => write!(f, "{}", step),
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Stash {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub method: StashMethod,
}

impl Stash {
    pub async fn run(&self, ctx: &StepContext, _job: &StepJob) -> Result<StepResponse> {
        let mut repo = ctx.git.lock().await;
        repo.stash_unstaged(false)?;
        Ok(Default::default())
    }
}

impl fmt::Display for Stash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Eq, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum StashMethod {
    #[default]
    GitDiff,
    GitStash,
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

        let repo = Git::new()?;
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
        } else if hook.name == "check" || hook.name == "fix" {
            // TODO: this should probably be customizable on the hook like `fix = true` is
            file_progress = file_progress_builder
                .prop("message", "Fetching modified files")
                .start();
            repo.modified_files()?
        } else {
            file_progress = file_progress_builder
                .prop("message", "Fetching staged files")
                .start();
            repo.staged_files()?
        };
        file_progress.prop("files", &files.len());
        file_progress.set_status(ProgressStatus::Done);
        let repo = Arc::new(Mutex::new(repo));

        let mut result = StepScheduler::new(hook, run_type, repo.clone())
            .with_files(files)
            .with_linters(linters)
            .with_tctx(tctx)
            .run()
            .await;

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
