use indexmap::IndexMap;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

use crate::{Result, cache::CacheManagerBuilder, env, hash, hook::Hook, version};
use eyre::{WrapErr, bail};

impl Config {
    #[tracing::instrument(level = "info", name = "config.load")]
    pub fn get() -> Result<Self> {
        let mut config = Self::load_project_config()?;
        config.apply_hkrc()?;
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
            "pkl" => run_pkl(&["eval"], path)?,
            _ => {
                bail!("Unsupported file extension: {}", ext);
            }
        };
        config.init(path)?;
        Ok(config)
    }

    /// Analyze pkl imports to get all transitive dependencies.
    /// Returns a set of local file paths that the config depends on.
    fn analyze_imports(path: &Path) -> Result<IndexSet<PathBuf>> {
        let imports: PklImports =
            run_pkl(&["analyze", "imports"], path).wrap_err("failed to analyze pkl")?;

        // Extract all local file paths from the imports map keys
        let mut paths = IndexSet::new();
        for uri in imports.resolvedImports.keys() {
            if let Some(file_path) = uri.strip_prefix("file://") {
                paths.insert(PathBuf::from(file_path));
            }
        }

        Ok(paths)
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
                // User-local config
                "hk.local.pkl",
                ".config/hk.local.pkl",
                // Standard config
                "hk.pkl",
                ".config/hk.pkl",
                // Soon-to-be-deprecated
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
                    return Self::load_config_cached(path);
                }
            }
            cwd = cwd.parent().map(PathBuf::from).unwrap_or_default();
        }
        debug!("No config file found, using default");
        let mut config = Config::default();
        config.init(Path::new(paths[0]))?;
        Ok(config)
    }

    fn load_config_cached(path: PathBuf) -> Result<Config> {
        let hash_key = format!("{}.json", hash::hash_to_str(&path));
        let cache_dir = env::HK_CACHE_DIR.join("configs");

        // For pkl files, we need to track all transitive imports for cache invalidation
        let is_pkl = path.extension().is_some_and(|ext| ext == "pkl");

        let fresh_files: Vec<PathBuf> = if is_pkl {
            // First, get the imports (cached separately, invalidated only by the main config file)
            let imports_cache_path =
                cache_dir.join(format!("{}-imports.json", hash::hash_to_str(&path)));
            let imports_cache_mgr = CacheManagerBuilder::new(imports_cache_path)
                .with_fresh_files(vec![path.clone()])
                .build::<IndexSet<PathBuf>>();

            let imports = imports_cache_mgr
                .get_or_try_init(|| Self::analyze_imports(&path))?
                .clone();

            imports.into_iter().collect()
        } else {
            vec![path.clone()]
        };

        // Build the config cache with all fresh files (imports + main config)
        let config_cache_path = cache_dir.join(hash_key);
        let config_cache_mgr = CacheManagerBuilder::new(config_cache_path)
            .with_fresh_files(fresh_files)
            .build::<Config>();

        // Load from cache if fresh; otherwise read from disk. In both cases, run init
        // to apply side-effects (env vars, settings, warnings) that are not stored in cache.
        let mut config = config_cache_mgr
            .get_or_try_init(|| {
                Self::read(&path)
                    .wrap_err_with(|| format!("Failed to read config file: {}", path.display()))
            })?
            .clone();
        config.init(&path)?;
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

    fn apply_hkrc(&mut self) -> Result<()> {
        let explicit_path = crate::settings::Settings::cli_user_config_path();

        let hkrc_path: Option<PathBuf> = if let Some(path) = explicit_path {
            // --hkrc was explicitly set: must exist
            if !path.exists() {
                bail!("Config file not found: {}", path.display());
            }
            Some(path)
        } else {
            // Default discovery: CWD, then $HOME, then XDG config dir
            let cwd_path = PathBuf::from(".hkrc.pkl");
            let home_path = env::HOME_DIR.join(".hkrc.pkl");
            let xdg_path = env::HK_CONFIG_DIR.join("config.pkl");
            if cwd_path.exists() {
                Some(cwd_path)
            } else if home_path.exists() {
                Some(home_path)
            } else if xdg_path.exists() {
                Some(xdg_path)
            } else {
                None
            }
        };

        if let Some(path) = hkrc_path {
            // Parse pkl output as raw JSON for format detection
            let json_value: serde_json::Value = run_pkl(&["eval"], &path)?;

            // Discriminate format: UserConfig.pkl has "environment", Config.pkl has "env"
            if json_value.get("environment").is_some() {
                let user_config: UserConfig = serde_json::from_value(json_value)
                    .wrap_err("failed to parse hkrc as UserConfig")?;
                self.apply_user_config(&Some(user_config))?;
            } else {
                let mut hkrc_config: Config = serde_json::from_value(json_value)
                    .wrap_err("failed to parse hkrc as Config")?;
                hkrc_config.init(&path)?;
                self.merge_from_hkrc(hkrc_config);
            }
        }
        Ok(())
    }

    fn merge_from_hkrc(&mut self, hkrc: Config) {
        // Environment: hkrc wins (matching existing apply_user_config behavior)
        for (key, value) in &hkrc.env {
            self.env.insert(key.clone(), value.clone());
            unsafe { std::env::set_var(key, value) };
        }

        // Scalar settings: project wins (only use hkrc value when project has None)
        if self.fail_fast.is_none() {
            self.fail_fast = hkrc.fail_fast;
        }
        if self.stage.is_none() {
            self.stage = hkrc.stage;
        }
        if self.display_skip_reasons.is_none() {
            self.display_skip_reasons = hkrc.display_skip_reasons;
        }
        if self.hide_warnings.is_none() {
            self.hide_warnings = hkrc.hide_warnings;
        }
        if self.warnings.is_none() {
            self.warnings = hkrc.warnings;
        }
        if self.exclude.is_none() {
            self.exclude = hkrc.exclude;
        }
        if self.profiles.is_none() {
            self.profiles = hkrc.profiles;
        }
        if self.skip_hooks.is_none() {
            self.skip_hooks = hkrc.skip_hooks;
        }
        if self.skip_steps.is_none() {
            self.skip_steps = hkrc.skip_steps;
        }
        if self.default_branch.is_none() {
            self.default_branch = hkrc.default_branch;
        }
        if self.min_hk_version.is_none() {
            self.min_hk_version = hkrc.min_hk_version;
        }

        // Hooks: additive, project wins on same-named step collision
        for (hook_name, hkrc_hook) in hkrc.hooks {
            if let Some(project_hook) = self.hooks.get_mut(&hook_name) {
                for (step_name, hkrc_step) in hkrc_hook.steps {
                    project_hook.steps.entry(step_name).or_insert(hkrc_step);
                }
            } else {
                self.hooks.insert(hook_name, hkrc_hook);
            }
        }
    }
}

/// Get the HTTP proxy address from environment variables.
/// Checks http_proxy, HTTP_PROXY, https_proxy, HTTPS_PROXY in that order.
fn get_http_proxy() -> Option<String> {
    std::env::var("http_proxy")
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTPS_PROXY"))
        .ok()
        .filter(|s| !s.is_empty())
}

/// Get the no_proxy list from environment variables.
/// Checks no_proxy and NO_PROXY.
fn get_no_proxy() -> Option<String> {
    std::env::var("no_proxy")
        .or_else(|_| std::env::var("NO_PROXY"))
        .ok()
        .filter(|s| !s.is_empty())
}

fn run_pkl<T: DeserializeOwned>(subcommand: &[&str], path: &Path) -> Result<T> {
    use std::process::{Command, Stdio};

    let try_run = |bin: &str| -> Result<T> {
        // Parse bin as shell words (e.g., "mise x -- pkl" -> ["mise", "x", "--", "pkl"])
        let bin_parts = shell_words::split(bin).wrap_err("failed to parse pkl command")?;
        let (cmd, bin_args) = bin_parts
            .split_first()
            .ok_or_else(|| eyre::eyre!("empty pkl command"))?;

        // Build pkl command args - flags must come before the positional path argument
        let mut args: Vec<String> = bin_args.to_vec();
        args.extend(subcommand.iter().map(|s| s.to_string()));
        args.extend(["-f".to_string(), "json".to_string()]);

        // Add --http-proxy if proxy env vars are set
        // Note: pkl only supports http:// proxies, not https:// proxy addresses
        if let Some(proxy) = get_http_proxy() {
            // pkl requires http:// scheme and doesn't support authentication
            if !proxy.starts_with("http://") {
                debug!("Ignoring proxy {proxy}: pkl only supports http:// proxies");
            } else if proxy.contains('@') {
                debug!("Ignoring proxy {proxy}: pkl does not support proxy authentication");
            } else {
                args.push("--http-proxy".to_string());
                args.push(proxy);
            }
        }

        // Add --http-no-proxy if no_proxy env var is set
        if let Some(no_proxy) = get_no_proxy() {
            args.push("--http-no-proxy".to_string());
            args.push(no_proxy);
        }

        if let Some(http_rewrite) = env::HK_PKL_HTTP_REWRITE.as_ref() {
            args.push("--http-rewrite".to_string());
            args.push(http_rewrite.to_string());
        }

        // Add the path last (positional argument must come after flags)
        args.push(path.display().to_string());

        // Run pkl directly without shell - safer and simpler
        let output = Command::new(cmd)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .wrap_err("failed to execute pkl command")?;

        if !output.status.success() {
            handle_pkl_error(&output, path)?;
        }

        let json = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&json).wrap_err("failed to parse pkl output")
    };

    match try_run("pkl") {
        Ok(result) => Ok(result),
        Err(err) => {
            // if pkl bin is not installed, try via mise
            if xx::file::which("pkl").is_none() {
                if let Ok(result) = try_run("mise x -- pkl") {
                    return Ok(result);
                }
                bail!("install pkl cli to use pkl config files https://pkl-lang.org/");
            }
            Err(err).wrap_err("failed to run pkl")
        }
    }
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
            • amends \"package://github.com/jdx/hk/releases/download/v1.36.0/hk@1.36.0#/Config.pkl\"",
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
    pub profiles: Option<Vec<String>>,
    pub skip_hooks: Option<Vec<String>>,
    pub skip_steps: Option<Vec<String>>,
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

/// Output of `pkl analyze imports -f json`
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct PklImports {
    resolvedImports: std::collections::HashMap<String, String>,
}
