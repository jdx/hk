use indexmap::IndexMap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

use crate::{Result, cache::CacheManagerBuilder, env, hash, hook::Hook, version};
use eyre::{WrapErr, bail};

impl Config {
    pub fn get() -> Result<Self> {
        let mut config = Self::load_project_config()?;
        let user_config = UserConfig::load()?;
        config.apply_user_config(&user_config)?;
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
        for (key, value) in self.env.iter() {
            unsafe { std::env::set_var(key, value) };
        }
        // Set display_skip_reasons if configured (None means use default, empty list means hide all)
        if let Some(display_skip_reasons) = &self.display_skip_reasons {
            crate::settings::Settings::set_display_skip_reasons(
                display_skip_reasons.clone().into_iter().collect(),
            );
        }
        // Set hide_warnings if configured
        if let Some(hide_warnings) = &self.hide_warnings {
            crate::settings::Settings::set_hide_warnings(
                hide_warnings.clone().into_iter().collect(),
            );
        }
        // Set warnings if configured
        if let Some(warnings) = &self.warnings {
            crate::settings::Settings::set_warnings(warnings.clone().into_iter().collect());
        }
        Ok(())
    }

    fn load_project_config() -> Result<Self> {
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

    fn apply_user_config(&mut self, user_config: &Option<UserConfig>) -> Result<()> {
        if let Some(user_config) = user_config {
            for (key, value) in &user_config.environment {
                // User config takes precedence over project config
                self.env.insert(key.clone(), value.clone());
                unsafe { std::env::set_var(key, value) };
            }

            if let Some(jobs) = user_config.defaults.jobs {
                if let Some(jobs) = std::num::NonZero::new(jobs as usize) {
                    crate::settings::Settings::set_jobs(jobs);
                }
            }

            if let Some(profiles) = &user_config.defaults.profiles {
                crate::settings::Settings::with_profiles(profiles);
            }

            if let Some(fail_fast) = user_config.defaults.fail_fast {
                crate::settings::Settings::set_fail_fast(fail_fast);
            }

            if let Some(all) = user_config.defaults.all {
                crate::settings::Settings::set_all(all);
            }

            if let Some(fix) = user_config.defaults.fix {
                crate::settings::Settings::set_fix(fix);
            }

            if let Some(check) = user_config.defaults.check {
                crate::settings::Settings::set_check(check);
            }

            if let Some(display_skip_reasons) = &user_config.display_skip_reasons {
                crate::settings::Settings::set_display_skip_reasons(
                    display_skip_reasons.clone().into_iter().collect(),
                );
            }

            if let Some(hide_warnings) = &user_config.hide_warnings {
                crate::settings::Settings::set_hide_warnings(
                    hide_warnings.clone().into_iter().collect(),
                );
            }
            if let Some(warnings) = &user_config.warnings {
                crate::settings::Settings::set_warnings(warnings.clone().into_iter().collect());
            }

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
                step.glob = Some(match glob {
                    StringOrList::String(s) => vec![s.clone()],
                    StringOrList::List(list) => list.clone(),
                });
            }

            if let Some(exclude) = &step_config.exclude {
                step.exclude = Some(match exclude {
                    StringOrList::String(s) => vec![s.clone()],
                    StringOrList::List(list) => list.clone(),
                });
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
        let user_config_path = crate::settings::Settings::get_user_config_path()
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
    #[serde(default)]
    pub env: IndexMap<String, String>,
    #[serde(rename = "display_skip_reasons")]
    pub display_skip_reasons: Option<Vec<String>>,
    #[serde(rename = "hide_warnings")]
    pub hide_warnings: Option<Vec<String>>,
    #[serde(rename = "warnings")]
    pub warnings: Option<Vec<String>>,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(self).unwrap())
    }
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        // TODO: validate config
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
    pub glob: Option<StringOrList>,
    pub exclude: Option<StringOrList>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrList {
    String(String),
    List(Vec<String>),
}
