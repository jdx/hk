use indexmap::IndexMap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

use crate::{Result, cache::CacheManagerBuilder, env, hash, hook::Hook, version};
use eyre::{WrapErr, bail};

impl Config {
    #[tracing::instrument(level = "info", name = "config.load")]
    pub fn get() -> Result<Self> {
        let mut config = Self::load_project_config()?;
        let user_config = UserConfig::load()?;
        config.apply_user_config(&user_config)?;
        config.validate()?;
        Ok(config)
    }

    #[tracing::instrument(level = "info", name = "config.read", skip_all, fields(path = %path.display()))]
    fn read(path: &Path) -> Result<Self> {
        let ext = path.extension().unwrap_or_default().to_str().unwrap();
        let mut config: Config = match ext {
            "toml" => {
                let raw = xx::file::read_to_string(path)?;
                toml::from_str(&raw)?
            }
            "yaml" | "yml" => {
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
            hook.init(name)?;
        }
        for (key, value) in self.env.iter() {
            unsafe { std::env::set_var(key, value) };
        }
        // No imperative settings mutation; values are consumed during Settings build
        Ok(())
    }

    #[tracing::instrument(level = "info", name = "config.load_project")]
    fn load_project_config() -> Result<Self> {
        let paths: Vec<&str> = if let Some(hk_file) = env::HK_FILE.as_ref() {
            // If HK_FILE is explicitly set, only use that path (no fallbacks)
            vec![hk_file.as_str()]
        } else {
            // Default search order when HK_FILE is not set
            vec![
                "hk.pkl",
                ".config/hk.pkl",
                "hk.toml",
                "hk.yaml",
                "hk.yml",
                "hk.json",
            ]
        };
        let mut cwd = std::env::current_dir()?;
        while cwd != Path::new("/") {
            for path in &paths {
                let path = cwd.join(path);
                if path.exists() {
                    let hash_key = format!("{}.json", hash::hash_to_str(&path));
                    let hash_key_path = env::HK_CACHE_DIR.join("configs").join(hash_key);
                    let cache_mgr = CacheManagerBuilder::new(hash_key_path)
                        .with_fresh_file(path.to_path_buf())
                        .build();
                    // Load from cache if fresh; otherwise read from disk. In both cases, run init
                    // to apply side-effects (env vars, settings, warnings) that are not stored in cache.
                    let mut config = cache_mgr
                        .get_or_try_init(|| {
                            Self::read(&path).wrap_err_with(|| {
                                format!("Failed to read config file: {}", path.display())
                            })
                        })?
                        .clone();
                    config.init(&path)?;
                    return Ok(config);
                }
            }
            cwd = cwd.parent().map(PathBuf::from).unwrap_or_default();
        }
        debug!("No config file found, using default");
        let mut config = Config::default();
        config.init(Path::new(paths[0]))?;
        Ok(config)
    }

    fn apply_user_config(&mut self, user_config: &Option<UserConfig>) -> Result<()> {
        if let Some(user_config) = user_config {
            // Top-level user settings that map to Settings should be copied so pkl map sees them
            if user_config.display_skip_reasons.is_some() {
                self.display_skip_reasons = user_config.display_skip_reasons.clone();
            }
            if user_config.hide_warnings.is_some() {
                self.hide_warnings = user_config.hide_warnings.clone();
            }
            if user_config.warnings.is_some() {
                self.warnings = user_config.warnings.clone();
            }
            if user_config.stage.is_some() {
                self.stage = user_config.stage
            }

            for (key, value) in &user_config.environment {
                // User config takes precedence over project config
                self.env.insert(key.clone(), value.clone());
                unsafe { std::env::set_var(key, value) };
            }

            // No imperative settings mutations here; Settings reads these during build

            for (hook_name, user_hook_config) in &user_config.hooks {
                if let Some(hook) = self.hooks.get_mut(hook_name) {
                    for (step_or_group_name, step_or_group) in hook.steps.iter_mut() {
                        match step_or_group {
                            crate::hook::StepOrGroup::Step(step) => {
                                let step_config = user_hook_config.steps.get(step_or_group_name);
                                Self::apply_user_config_to_step(
                                    step,
                                    user_hook_config,
                                    step_config,
                                )?;
                            }
                            crate::hook::StepOrGroup::Group(group) => {
                                for (step_name, step) in group.steps.iter_mut() {
                                    let step_config = user_hook_config.steps.get(step_name);
                                    Self::apply_user_config_to_step(
                                        step,
                                        user_hook_config,
                                        step_config,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_user_config_to_step(
        step: &mut crate::step::Step,
        hook_config: &UserHookConfig,
        step_config: Option<&UserStepConfig>,
    ) -> Result<()> {
        for (key, value) in &hook_config.environment {
            step.env.entry(key.clone()).or_insert_with(|| value.clone());
        }

        if let Some(step_config) = step_config {
            for (key, value) in &step_config.environment {
                step.env.entry(key.clone()).or_insert_with(|| value.clone());
            }

            if let Some(glob) = &step_config.glob {
                step.glob = Some(glob.clone());
            }

            if let Some(exclude) = &step_config.exclude {
                step.exclude = Some(exclude.clone());
            }

            if let Some(profiles) = &step_config.profiles {
                step.profiles = Some(profiles.clone());
            }
        }

        Ok(())
    }
}

impl UserConfig {
    fn load() -> Result<Option<Self>> {
        let user_config_path = crate::settings::Settings::cli_user_config_path()
            .expect("Config path should always be set by CLI");

        if user_config_path.exists() {
            let user_config: UserConfig = parse_pkl("pkl", &user_config_path)?;
            Ok(Some(user_config))
        } else {
            let default_path = PathBuf::from(".hkrc.pkl");
            if user_config_path != default_path {
                bail!("Config file not found: {}", user_config_path.display());
            }
            Ok(None)
        }
    }
}

fn parse_pkl<T: DeserializeOwned>(bin: &str, path: &Path) -> Result<T> {
    use std::process::{Command, Stdio};

    // Run pkl with captured stderr to check for specific error patterns
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("{bin} eval -f json {}", path.display()))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .wrap_err("failed to execute pkl command")?;

    if !output.status.success() {
        handle_pkl_error(&output, path)?;
    }

    let json = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json).wrap_err("failed to parse pkl config file")
}

fn handle_pkl_error(output: &std::process::Output, path: &Path) -> Result<()> {
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Check for common Pkl errors and provide helpful messages
    if stderr.contains("Cannot find type `Hook`") || stderr.contains("Cannot find type `Step`") {
        bail!(
            "Missing 'amends' declaration in {}. \n\n\
            Your hk.pkl file should start with one of:\n\
            • amends \"pkl/Config.pkl\" (if vendored)\n\
            • amends \"package://github.com/jdx/hk/releases/download/vX.Y.Z/hk@X.Y.Z#/Config.pkl\" (for released versions)\n\n\
            See https://github.com/jdx/hk for more information.",
            path.display()
        );
    } else if stderr.contains("Module URI") && stderr.contains("has invalid syntax") {
        bail!(
            "Invalid module URI in {}. \n\n\
            Make sure your 'amends' declaration uses a valid path or package URL.\n\
            Examples:\n\
            • amends \"pkl/Config.pkl\" (if vendored)\n\
            • amends \"package://github.com/jdx/hk/releases/download/v1.22.0/hk@1.22.0#/Config.pkl\"",
            path.display()
        );
    }

    // Return the full error if it's not a known pattern
    let code = output
        .status
        .code()
        .map_or("unknown".to_string(), |c| c.to_string());
    bail!(
        "Failed to evaluate Pkl config at {}\n\nExit code: {}\n\nError output:\n{}",
        path.display(),
        code,
        stderr
    );
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct Config {
    pub min_hk_version: Option<String>,
    #[serde(default)]
    pub hooks: IndexMap<String, Hook>,
    /// Preferred default branch to compare against (e.g. "main"). If not set, hk will detect it.
    pub default_branch: Option<String>,
    #[serde(skip)]
    #[serde(default)]
    pub path: PathBuf,
    #[serde(default)]
    pub env: IndexMap<String, String>,
    pub fail_fast: Option<bool>,
    pub display_skip_reasons: Option<Vec<String>>,
    pub hide_warnings: Option<Vec<String>>,
    pub warnings: Option<Vec<String>>,
    /// Global file patterns to exclude from all steps
    pub exclude: Option<StringOrList>,
    pub stage: Option<bool>,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(self).unwrap())
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        // Validate that steps with 'stage' attribute also have a 'fix' command
        for (hook_name, hook) in &self.hooks {
            for (step_name, step_or_group) in &hook.steps {
                match step_or_group {
                    crate::hook::StepOrGroup::Step(step) => {
                        if step.stage.is_some() && step.fix.is_none() {
                            bail!(
                                "Step '{}' in hook '{}' has 'stage' attribute but no 'fix' command. \
                                Steps that stage files must have a fix command.",
                                step_name,
                                hook_name
                            );
                        }
                    }
                    crate::hook::StepOrGroup::Group(group) => {
                        for (group_step_name, group_step) in &group.steps {
                            if group_step.stage.is_some() && group_step.fix.is_none() {
                                bail!(
                                    "Step '{}' in group '{}' of hook '{}' has 'stage' attribute but no 'fix' command. \
                                    Steps that stage files must have a fix command.",
                                    group_step_name,
                                    step_name,
                                    hook_name
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UserConfig {
    #[serde(default)]
    pub environment: IndexMap<String, String>,
    #[serde(default)]
    pub defaults: UserDefaults,
    #[serde(default)]
    pub hooks: IndexMap<String, UserHookConfig>,
    #[serde(rename = "display_skip_reasons")]
    pub display_skip_reasons: Option<Vec<String>>,
    #[serde(rename = "hide_warnings")]
    pub hide_warnings: Option<Vec<String>>,
    #[serde(rename = "warnings")]
    pub warnings: Option<Vec<String>>,
    pub stage: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UserDefaults {
    pub jobs: Option<u16>,
    pub fail_fast: Option<bool>,
    pub profiles: Option<Vec<String>>,
    pub all: Option<bool>,
    pub fix: Option<bool>,
    pub check: Option<bool>,
    pub exclude: Option<StringOrList>,
    pub skip_steps: Option<StringOrList>,
    pub skip_hooks: Option<StringOrList>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UserHookConfig {
    #[serde(default)]
    pub environment: IndexMap<String, String>,
    pub jobs: Option<u16>,
    pub fail_fast: Option<bool>,
    pub profiles: Option<Vec<String>>,
    pub all: Option<bool>,
    pub fix: Option<bool>,
    pub check: Option<bool>,
    #[serde(default)]
    pub steps: IndexMap<String, UserStepConfig>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct UserStepConfig {
    #[serde(default)]
    pub environment: IndexMap<String, String>,
    pub fail_fast: Option<bool>,
    pub profiles: Option<Vec<String>>,
    pub all: Option<bool>,
    pub fix: Option<bool>,
    pub check: Option<bool>,
    pub glob: Option<crate::step::Pattern>,
    pub exclude: Option<crate::step::Pattern>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrList {
    String(String),
    List(Vec<String>),
}

impl IntoIterator for StringOrList {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            StringOrList::String(s) => vec![s].into_iter(),
            StringOrList::List(list) => list.into_iter(),
        }
    }
}
