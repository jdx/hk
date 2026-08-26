use indexmap::IndexMap;
use indexmap::IndexSet;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::future::Future;
use std::path::{Path, PathBuf};

use crate::{Result, cache::CacheManagerBuilder, env, hash, hook::Hook, version};
use eyre::{WrapErr, bail};

impl Config {
    /// Return the resolved config for this hk invocation.
    ///
    /// An hk process handles a single command in a single project, so config
    /// resolution only needs to happen once. Callers receive a clone so their
    /// mutations cannot affect later callers.
    pub fn get() -> Result<Self> {
        static RESOLVED_CONFIG: OnceCell<Config> = OnceCell::new();
        Ok(RESOLVED_CONFIG.get_or_try_init(Self::load)?.clone())
    }

    #[tracing::instrument(level = "info", name = "config.load")]
    fn load() -> Result<Self> {
        if std::env::var_os("HK_PKL_BACKEND").is_some() {
            bail!(
                "HK_PKL_BACKEND was removed in hk v2; hk now always uses the built-in pklr evaluator. Remove this environment variable"
            );
        }
        let mut config = Self::load_project_config()?;
        config.load_subprojects()?;
        config.materialize_default_hooks()?;
        config.apply_hkrc()?;
        config.validate()?;
        Ok(config)
    }

    #[tracing::instrument(level = "info", name = "config.read", skip_all, fields(path = %path.display()))]
    fn read(path: &Path, apply_env: bool) -> Result<Self> {
        let ext = path.extension().unwrap_or_default().to_str().unwrap();
        let mut config: Config = match ext {
            "pkl" => run_pklr(path)?,
            "toml" | "yaml" | "yml" | "json" => bail!(
                "{} configuration was removed in hk v2; convert {} to hk.pkl and amend Config.pkl",
                ext.to_uppercase(),
                path.display()
            ),
            _ => bail!("Unsupported config extension: {ext}; hk v2 requires a Pkl config"),
        };
        config.init(path, apply_env)?;
        Ok(config)
    }

    /// Analyze pkl imports to get all transitive dependencies.
    /// Returns local file paths that the config depends on and whether the
    /// module graph contains imports whose bytes hk cannot hash.
    fn analyze_imports(path: &Path) -> Result<ImportAnalysis> {
        let local_paths: IndexSet<PathBuf> = block_on_pklr(pklr::analyze_imports_async(path))?
            .map(|v| v.into_iter().collect())
            .map_err(|e| eyre::eyre!("{e}"))?;
        let has_untracked_imports = Self::has_untracked_imports_in_pkl_sources(path, &local_paths)?;

        Ok(ImportAnalysis {
            local_paths,
            has_untracked_imports,
        })
    }

    fn has_untracked_imports_in_pkl_sources(
        path: &Path,
        local_paths: &IndexSet<PathBuf>,
    ) -> Result<bool> {
        let mut paths = IndexSet::new();
        paths.insert(path.to_path_buf());
        paths.extend(local_paths.iter().cloned());
        for path in paths {
            let source = std::fs::read_to_string(&path)
                .wrap_err_with(|| format!("failed to read pkl imports from {}", path.display()))?;
            if Self::source_may_reference_untracked_import(&source) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn source_may_reference_untracked_import(source: &str) -> bool {
        source.lines().map(str::trim_start).any(|line| {
            !line.starts_with("//")
                && ["amends", "extends", "import", "import*"]
                    .iter()
                    .any(|keyword| line.starts_with(keyword))
                && ["\"http://", "\"https://", "\"package://"]
                    .iter()
                    .any(|scheme| line.contains(scheme))
        })
    }

    fn init(&mut self, path: &Path, apply_env: bool) -> Result<()> {
        self.path = path.to_path_buf();
        if let Some(min_hk_version) = &self.min_hk_version {
            version::version_cmp_or_bail(min_hk_version)?;
        }
        for (name, hook) in self.hooks.iter_mut() {
            hook.init(name)?;
        }
        // Subproject configs keep their env scoped to their own steps, so only
        // the root config exports env vars to the hk process itself.
        if apply_env {
            for (key, value) in self.env.iter() {
                unsafe { std::env::set_var(key, value) };
            }
        }
        // No imperative settings mutation; values are consumed during Settings build
        Ok(())
    }

    #[tracing::instrument(level = "info", name = "config.load_project")]
    fn load_project_config() -> Result<Self> {
        let paths = Self::project_config_search_paths();
        if let Some(path) = Self::find_project_config(&paths) {
            return Self::load_config_cached(path);
        }
        debug!("No config file found, using default");
        let mut config = Config::default();
        config.init(Path::new(&paths[0]), true)?;
        Ok(config)
    }

    fn project_config_search_paths() -> Vec<String> {
        if let Some(hk_file) = env::HK_FILE.as_ref() {
            // If HK_FILE is explicitly set, only use that path (no fallbacks)
            vec![hk_file.clone()]
        } else {
            [
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
            .iter()
            .map(|s| s.to_string())
            .collect()
        }
    }

    fn find_project_config(paths: &[String]) -> Option<PathBuf> {
        let cwd = std::env::current_dir().ok()?;
        Self::find_project_config_from(&cwd, paths)
    }

    fn find_project_config_from(start: &Path, paths: &[String]) -> Option<PathBuf> {
        let mut cwd = start.to_path_buf();
        while cwd.parent().is_some() {
            for name in paths {
                let p = cwd.join(name);
                if p.exists() {
                    return Some(p);
                }
            }
            cwd = cwd.parent().map(PathBuf::from).unwrap_or_default();
        }
        None
    }

    /// Returns true when a project-level hk config file exists without
    /// loading or parsing it. Used by `--from-hook` so a broken user-global
    /// hkrc doesn't blow up `git commit` in repos that have no hk.pkl.
    pub fn project_config_exists() -> bool {
        Self::find_project_config(&Self::project_config_search_paths()).is_some()
    }

    /// Returns true when project config discovery from `start` would find a
    /// config in that directory or one of its ancestors.
    pub fn project_config_exists_from(start: &Path) -> bool {
        Self::find_project_config_from(start, &Self::project_config_search_paths()).is_some()
    }

    fn load_config_cached(path: PathBuf) -> Result<Config> {
        Self::load_config_cached_with(path, true)
    }

    /// Load a config file with caching. `is_root` controls whether the config's
    /// `env` is exported to the hk process (root config only) and whether the
    /// shared `resolved-config.json` cache slot may be used — subproject configs
    /// always get a path-keyed cache file so they don't thrash the root's slot.
    fn load_config_cached_with(path: PathBuf, is_root: bool) -> Result<Config> {
        let hash_key = format!("{}.json", hash::hash_to_str(&path));
        let cache_dir = env::HK_CACHE_DIR.join("configs");

        // For pkl files, we need to track all transitive imports for cache invalidation
        let is_pkl = path.extension().is_some_and(|ext| ext == "pkl");

        let (fresh_files, has_untracked_imports): (Vec<PathBuf>, bool) = if is_pkl {
            // First, get the imports (cached separately, invalidated only by the main config file)
            let imports_cache_path =
                cache_dir.join(format!("{}-imports.json", hash::hash_to_str(&path)));
            let imports_cache_mgr = CacheManagerBuilder::new(imports_cache_path)
                .with_fresh_files(vec![path.clone()])
                .build::<ImportAnalysis>();

            let import_analysis = imports_cache_mgr
                .get_or_try_init(|| Self::analyze_imports(&path))?
                .clone();
            let has_untracked_imports = import_analysis.has_untracked_imports
                || Self::has_untracked_imports_in_pkl_sources(&path, &import_analysis.local_paths)?;

            // Always include the main config file. pklr's analyze_imports does
            // not include the source file in its output, so without this edits
            // to hk.pkl would not invalidate the cache.
            let mut files: IndexSet<PathBuf> = import_analysis.local_paths;
            files.insert(path.clone());
            (files.into_iter().collect(), has_untracked_imports)
        } else {
            (vec![path.clone()], false)
        };

        // Build the config cache with all fresh files (imports + main config)
        let config_cache_path = if has_untracked_imports || !is_root {
            cache_dir.join(hash_key)
        } else {
            cache_dir.join("resolved-config.json")
        };
        let config_cache_builder = CacheManagerBuilder::new(config_cache_path);
        let config_cache_mgr = if has_untracked_imports {
            config_cache_builder.with_fresh_files(fresh_files)
        } else {
            config_cache_builder.with_content_fresh_files(fresh_files)
        }
        .build::<Config>();

        // Load from cache if fresh; otherwise read from disk. In both cases, run init
        // to apply side-effects (env vars, settings, warnings) that are not stored in cache.
        let mut config = config_cache_mgr
            .get_or_try_init(|| {
                Self::read(&path, is_root)
                    .wrap_err_with(|| format!("Failed to read config file: {}", path.display()))
            })?
            .clone();
        config.init(&path, is_root)?;
        Ok(config)
    }

    fn apply_hkrc(&mut self) -> Result<()> {
        let cwd_path = PathBuf::from(".hkrc.pkl");
        if cwd_path.exists() {
            bail!(
                ".hkrc.pkl was removed in hk v2; rename it to hk.local.pkl for project overrides"
            );
        }
        let home_path = env::HOME_DIR.join(".hkrc.pkl");
        if home_path.exists() {
            bail!(
                "~/.hkrc.pkl was removed in hk v2; move it to {}/config.pkl",
                env::HK_CONFIG_DIR.display()
            );
        }

        let xdg_path = env::HK_CONFIG_DIR.join("config.pkl");
        let hkrc_path = xdg_path.exists().then_some(xdg_path);

        if let Some(path) = hkrc_path {
            // Parse pkl output as raw JSON for format detection
            let json_value: serde_json::Value = run_pklr(&path)?;

            if json_value.get("environment").is_some() {
                bail!(
                    "UserConfig.pkl and its `environment` field were removed in hk v2; amend Config.pkl and rename `environment` to `env`"
                );
            }
            let mut hkrc_config: Config = serde_json::from_value(json_value)
                .wrap_err("failed to parse global config as Config")?;
            // The project config has already exported its environment. Do not
            // overwrite it before the project-wins merge below.
            hkrc_config.init(&path, false)?;
            hkrc_config.materialize_default_hooks()?;
            self.merge_from_hkrc(hkrc_config);
        }
        Ok(())
    }

    fn merge_from_hkrc(&mut self, hkrc: Config) {
        // Environment: project wins. hkrc values are set only if not defined by project.
        // set_var is unsafe in Rust 2024 but required so child processes inherit these.
        for (key, value) in hkrc.env {
            if let indexmap::map::Entry::Vacant(e) = self.env.entry(key.clone()) {
                unsafe { std::env::set_var(&key, &value) };
                e.insert(value);
            }
        }

        // Scalar settings: project wins — fall back to hkrc when project has None
        self.fail_fast = self.fail_fast.or(hkrc.fail_fast);
        self.stage = self.stage.or(hkrc.stage);
        self.display_skip_reasons = self
            .display_skip_reasons
            .take()
            .or(hkrc.display_skip_reasons);
        self.hide_warnings = self.hide_warnings.take().or(hkrc.hide_warnings);
        self.warnings = self.warnings.take().or(hkrc.warnings);
        self.exclude = self.exclude.take().or(hkrc.exclude);
        self.profiles = self.profiles.take().or(hkrc.profiles);
        self.skip_hooks = self.skip_hooks.take().or(hkrc.skip_hooks);
        self.skip_steps = self.skip_steps.take().or(hkrc.skip_steps);
        self.default_branch = self.default_branch.take().or(hkrc.default_branch);
        self.min_hk_version = self.min_hk_version.take().or(hkrc.min_hk_version);

        // Top-level steps are additive, with project definitions winning.
        for (step_name, hkrc_step) in hkrc.steps {
            self.steps.entry(step_name).or_insert(hkrc_step);
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

    /// Load configs from `subprojects` directories and merge their hooks into
    /// this config, scoped to each subdirectory. Entries may be literal
    /// directories ("subproject") or glob patterns ("packages/*").
    fn load_subprojects(&mut self) -> Result<()> {
        let Some(patterns) = self.subprojects.clone() else {
            return Ok(());
        };
        if patterns.is_empty() {
            return Ok(());
        }
        for pattern in &patterns {
            let p = Path::new(pattern);
            if p.is_absolute()
                || p.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                bail!("subprojects entries must be relative paths without '..': {pattern}");
            }
        }
        let root = Self::project_root_of(&self.path);
        for (dir, config_path) in Self::discover_subprojects(&root, &patterns)? {
            debug!("loading subproject config: {}", config_path.display());
            let sub = Self::load_config_cached_with(config_path.clone(), false)?;
            self.merge_subproject(&dir, sub).wrap_err_with(|| {
                format!(
                    "failed to merge subproject config: {}",
                    config_path.display()
                )
            })?;
        }
        Ok(())
    }

    /// The project root a config file belongs to. Configs may live at the root
    /// itself (hk.pkl) or under a `.config/` directory (.config/hk.pkl), in
    /// which case the root is one level further up.
    fn project_root_of(config_path: &Path) -> PathBuf {
        let parent = config_path.parent().filter(|p| !p.as_os_str().is_empty());
        let parent = match parent {
            Some(p) => p,
            None => return PathBuf::from("."),
        };
        if parent.file_name().is_some_and(|name| name == ".config") {
            parent
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            parent.to_path_buf()
        }
    }

    /// Resolve `subprojects` entries to (relative dir, config file) pairs.
    /// Literal entries warn when missing; glob matches without a config file
    /// are silently skipped.
    fn discover_subprojects(root: &Path, patterns: &[String]) -> Result<Vec<(String, PathBuf)>> {
        let is_glob = |p: &str| p.chars().any(|c| matches!(c, '*' | '?' | '[' | '{'));
        let mut out: IndexMap<String, PathBuf> = IndexMap::new();
        // Walk the tree once (bounded by the deepest glob) if any globs are present
        let glob_patterns = patterns.iter().filter(|p| is_glob(p)).collect::<Vec<_>>();
        let walked: Vec<String> = if glob_patterns.is_empty() {
            vec![]
        } else {
            // `**` patterns get a generous but bounded depth so a pathological
            // tree can't make discovery walk forever
            const MAX_WALK_DEPTH: usize = 32;
            let max_depth = glob_patterns
                .iter()
                .map(|p| {
                    if p.contains("**") {
                        MAX_WALK_DEPTH
                    } else {
                        p.split('/').count()
                    }
                })
                .max()
                .unwrap();
            let mut dirs = vec![];
            Self::walk_dirs(root, root, max_depth, &mut dirs);
            dirs.sort();
            dirs
        };
        for pattern in patterns {
            if is_glob(pattern) {
                let matcher = globset::GlobBuilder::new(pattern)
                    .literal_separator(true)
                    .empty_alternates(true)
                    .build()
                    .wrap_err_with(|| format!("invalid subprojects glob: {pattern}"))?
                    .compile_matcher();
                for dir in walked.iter().filter(|d| matcher.is_match(d)) {
                    if out.contains_key(dir) {
                        continue;
                    }
                    if let Some(config_path) = Self::find_subproject_config(&root.join(dir)) {
                        out.insert(dir.clone(), config_path);
                    } else {
                        debug!("subprojects: no hk config in {dir}, skipping");
                    }
                }
            } else {
                let dir = pattern.trim_end_matches('/').to_string();
                if !root.join(&dir).is_dir() {
                    warn!("subprojects: directory not found: {dir}");
                    continue;
                }
                if out.contains_key(&dir) {
                    continue;
                }
                match Self::find_subproject_config(&root.join(&dir)) {
                    Some(config_path) => {
                        out.insert(dir, config_path);
                    }
                    None => warn!("subprojects: no hk config found in {dir}"),
                }
            }
        }
        Ok(out.into_iter().collect())
    }

    /// Recursively collect directories (relative, '/'-separated) up to max_depth,
    /// skipping hidden directories, node_modules, and symlinks.
    fn walk_dirs(root: &Path, dir: &Path, max_depth: usize, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            let path = entry.path();
            if path.is_symlink() || !path.is_dir() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            let depth = rel.split('/').count();
            if depth <= max_depth {
                out.push(rel);
            }
            if depth < max_depth {
                Self::walk_dirs(root, &path, max_depth, out);
            }
        }
    }

    fn find_subproject_config(dir: &Path) -> Option<PathBuf> {
        [
            "hk.local.pkl",
            ".config/hk.local.pkl",
            "hk.pkl",
            ".config/hk.pkl",
            "hk.toml",
            "hk.yaml",
            "hk.yml",
            "hk.json",
        ]
        .iter()
        .map(|name| dir.join(name))
        .find(|p| p.exists())
    }

    /// Merge a subproject config into this one. Each hook's steps are scoped to
    /// `subdir`: working directories are joined onto the subdirectory (which also
    /// scopes glob matching), step/group names are prefixed with "{subdir}:", and
    /// the subproject's `env` is applied to its own steps only.
    fn merge_subproject(&mut self, subdir: &str, mut sub: Config) -> Result<()> {
        sub.materialize_default_hooks()?;
        if sub.subprojects.as_ref().is_some_and(|s| !s.is_empty()) {
            warn!(
                "subprojects: nested `subprojects` in {} is ignored (only one level is supported)",
                sub.path.display()
            );
        }
        let sub_env = std::mem::take(&mut sub.env);
        for (hook_name, sub_hook) in std::mem::take(&mut sub.hooks) {
            if !sub_hook.enabled {
                continue;
            }
            let root_hook = self.hooks.entry(hook_name.clone()).or_insert_with(|| Hook {
                name: hook_name.clone(),
                fix: sub_hook.fix,
                stash: sub_hook.stash.clone(),
                stage: sub_hook.stage,
                fail_on_fix: sub_hook.fail_on_fix,
                report: sub_hook.report.clone(),
                ..Default::default()
            });
            // Names of the subproject hook's steps and groups, for rewriting
            // `depends` references to their scoped names.
            let sibling_names = sub_hook.steps.keys().cloned().collect::<IndexSet<_>>();
            for (name, step_or_group) in sub_hook.steps {
                let scoped_name = format!("{subdir}:{name}");
                if root_hook.steps.contains_key(&scoped_name) {
                    bail!("duplicate step name '{scoped_name}' in hook '{hook_name}'");
                }
                let step_or_group = match step_or_group {
                    crate::hook::StepOrGroup::Step(mut step) => {
                        Self::scope_subproject_step(&mut step, subdir, &sub_hook.env, &sub_env);
                        step.name = scoped_name.clone();
                        step.depends = step
                            .depends
                            .iter()
                            .map(|dep| {
                                if sibling_names.contains(dep) {
                                    format!("{subdir}:{dep}")
                                } else {
                                    dep.clone()
                                }
                            })
                            .collect();
                        crate::hook::StepOrGroup::Step(step)
                    }
                    crate::hook::StepOrGroup::Group(mut group) => {
                        group.name = Some(scoped_name.clone());
                        group.dir = Some(Self::join_subdir(subdir, group.dir.as_deref()));
                        for step in group.steps.values_mut() {
                            Self::scope_subproject_step(step, subdir, &sub_hook.env, &sub_env);
                        }
                        crate::hook::StepOrGroup::Group(group)
                    }
                };
                root_hook.steps.insert(scoped_name, step_or_group);
            }
        }
        Ok(())
    }

    fn scope_subproject_step(
        step: &mut crate::step::Step,
        subdir: &str,
        hook_env: &IndexMap<String, String>,
        config_env: &IndexMap<String, String>,
    ) {
        step.dir = Some(Self::join_subdir(subdir, step.dir.as_deref()));
        // step env wins over the subproject's hook env, which wins over its config env
        for (key, value) in hook_env.iter().chain(config_env.iter()) {
            step.env.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }

    fn join_subdir(subdir: &str, dir: Option<&str>) -> String {
        match dir {
            Some(dir) if !dir.is_empty() => format!("{subdir}/{dir}"),
            _ => subdir.to_string(),
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

/// The pkl package for this version, staged by `build/embed_pkl_package.rs`.
/// Empty when the pkl sources were not generated.
static EMBEDDED_PKL_PACKAGE: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/hk_pkl_package.zip"));

/// The release archive URL `hk init` writes into `hk.pkl` for this version.
fn embedded_pkl_package_url() -> String {
    let version = version::version();
    format!("https://github.com/jdx/hk/releases/download/v{version}/hk@{version}.zip")
}

fn run_pklr<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let client = build_pklr_http_client()?;
    let http_rewrites = env::HK_PKL_HTTP_REWRITE
        .as_deref()
        .map(|s| s.split(',').map(String::from).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut evaluator = pklr::AsyncEvaluatorBuilder::new()
        .http_client(client)
        .http_rewrites(http_rewrites)
        .package_cache_dir(env::HK_PKL_CACHE_DIR.clone())
        .offline(*env::HK_PKL_OFFLINE);
    // A config pinning this version then needs no network on a cold cache; any
    // other version keys a different URL and is fetched as usual.
    if *env::HK_PKL_EMBEDDED && !EMBEDDED_PKL_PACKAGE.is_empty() {
        evaluator =
            evaluator.preload_package(embedded_pkl_package_url(), "zip", EMBEDDED_PKL_PACKAGE);
    }
    let json = block_on_pklr(evaluator.eval_to_json(path))?
        .map_err(|e| handle_pklr_eval_error(&e.to_string(), path))?;
    serde_json::from_value(json).map_err(|e| handle_pklr_deserialize_error(&e.to_string(), path))
}

fn block_on_pklr<T>(future: impl Future<Output = pklr::Result<T>>) -> Result<pklr::Result<T>> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => Ok(tokio::task::block_in_place(|| handle.block_on(future))),
        Err(_) => tokio::runtime::Runtime::new()
            .map(|runtime| runtime.block_on(future))
            .map_err(Into::into),
    }
}

/// Build a reqwest::Client with proxy and CA certificate settings
/// matching proxy and HK_PKL_* environment variables.
fn build_pklr_http_client() -> Result<pklr::reqwest::Client> {
    let mut builder = pklr::reqwest::Client::builder();
    if let Some(proxy_url) = get_http_proxy() {
        let mut proxy = pklr::reqwest::Proxy::all(&proxy_url)
            .map_err(|e| eyre::eyre!("invalid proxy URL: {e}"))?;
        if let Some(no_proxy) = get_no_proxy() {
            proxy = proxy.no_proxy(pklr::reqwest::NoProxy::from_string(&no_proxy));
        }
        builder = builder.proxy(proxy);
    }
    if let Some(ca_path) = env::HK_PKL_CA_CERTIFICATES.as_ref() {
        let cert_pem = std::fs::read(ca_path)
            .map_err(|e| eyre::eyre!("failed to read CA certificate {}: {e}", ca_path.display()))?;
        let certs = pklr::reqwest::Certificate::from_pem_bundle(&cert_pem)
            .map_err(|e| eyre::eyre!("invalid CA certificate: {e}"))?;
        for cert in certs {
            builder = builder.add_root_certificate(cert);
        }
    }
    builder
        .build()
        .map_err(|e| eyre::eyre!("failed to build HTTP client: {e}"))
}

/// Get the no_proxy list from environment variables.
/// Checks no_proxy and NO_PROXY.
fn get_no_proxy() -> Option<String> {
    std::env::var("no_proxy")
        .or_else(|_| std::env::var("NO_PROXY"))
        .ok()
        .filter(|s| !s.is_empty())
}

fn handle_pklr_eval_error(error: &str, path: &Path) -> eyre::Report {
    if error.contains("unsupported package URI")
        || (error.contains("Module URI") && error.contains("has invalid syntax"))
    {
        return invalid_module_uri_error(path);
    }
    failed_pkl_config_error(path, error)
}

fn handle_pklr_deserialize_error(error: &str, path: &Path) -> eyre::Report {
    if !pkl_file_has_amends(path) && error.contains("unknown field") {
        return missing_amends_error(path);
    }
    failed_pkl_config_error(path, error)
}

fn pkl_file_has_amends(path: &Path) -> bool {
    xx::file::read_to_string(path).ok().is_some_and(|raw| {
        raw.lines()
            .any(|line| line.trim_start().starts_with("amends "))
    })
}

fn missing_amends_error(path: &Path) -> eyre::Report {
    let version = env!("CARGO_PKG_VERSION");
    eyre::eyre!(
        "Missing 'amends' declaration in {}. \n\n\
        Your hk.pkl file should start with one of:\n\
        • amends \"pkl/Config.pkl\" (if vendored)\n\
        • amends \"package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Config.pkl\" (for released versions)\n\n\
        See https://github.com/jdx/hk for more information.",
        path.display()
    )
}

fn invalid_module_uri_error(path: &Path) -> eyre::Report {
    let version = env!("CARGO_PKG_VERSION");
    eyre::eyre!(
        "Invalid module URI in {}. \n\n\
        Make sure your 'amends' declaration uses a valid path or package URL.\n\
        Examples:\n\
        • amends \"pkl/Config.pkl\" (if vendored)\n\
        • amends \"package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Config.pkl\"",
        path.display()
    )
}

fn failed_pkl_config_error(path: &Path, stderr: &str) -> eyre::Report {
    let source = std::fs::read_to_string(path).unwrap_or_default();
    let mut hints = Vec::new();
    let combined = format!("{source}\n{stderr}");
    let uses_legacy_environment = source.contains("environment {")
        || source.contains("environment=")
        || source.contains("environment =");
    if combined.contains("UserConfig.pkl") || uses_legacy_environment {
        hints.push(
            "UserConfig.pkl was removed in hk v2; amend Config.pkl and rename `environment` to `env`.",
        );
    }
    if combined.contains("Types.pkl") || combined.contains("Types.Regex") {
        hints.push("Types.pkl was removed in hk v2; use Pkl's built-in `Regex` type directly.");
    }
    if combined.contains("Config.Regex") {
        hints.push("Config.Regex was removed in hk v2; use Pkl's built-in `Regex` directly.");
    }
    if combined.contains("check_byte_order_marker") {
        hints.push(
            "Builtins.check_byte_order_marker was removed in hk v2; use Builtins.byte_order_marker.",
        );
    }
    if combined.contains("fix_byte_order_marker") {
        hints.push(
            "Builtins.fix_byte_order_marker was removed in hk v2; use Builtins.byte_order_marker.",
        );
    }
    let hint = if hints.is_empty() {
        String::new()
    } else {
        format!("\n\nMigration:\n- {}", hints.join("\n- "))
    };
    eyre::eyre!(
        "Failed to evaluate Pkl config at {}\n\nError output:\n{}{}",
        path.display(),
        stderr,
        hint
    )
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(debug_assertions, serde(deny_unknown_fields))]
pub struct Config {
    pub min_hk_version: Option<String>,
    #[serde(default)]
    pub steps: IndexMap<String, crate::hook::StepOrGroup>,
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
    /// Directories (or glob patterns) containing their own hk config files.
    /// Their hooks are merged into this config, scoped to the subdirectory.
    pub subprojects: Option<Vec<String>>,
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", toml::to_string(self).unwrap())
    }
}

impl Config {
    fn materialize_default_hooks(&mut self) -> Result<()> {
        if self.steps.is_empty() {
            return Ok(());
        }

        for (name, fix, stage, stash) in [
            ("check", Some(false), Some(false), None),
            ("fix", Some(true), Some(false), None),
            (
                "pre-commit",
                Some(true),
                Some(true),
                Some(crate::hook::StashSetting::Method(
                    crate::git::StashMethod::Git,
                )),
            ),
        ] {
            let hook = self.hooks.entry(name.to_string()).or_default();
            let explicit_steps = std::mem::take(&mut hook.steps);
            hook.steps = self.steps.clone();
            hook.steps.extend(explicit_steps);
            hook.fix = hook.fix.or(fix);
            hook.stage = hook.stage.or(stage);
            hook.stash = hook.stash.clone().or(stash);
            hook.init(name)?;
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        for (hook_name, hook) in &self.hooks {
            for (step_name, step_or_group) in &hook.steps {
                match step_or_group {
                    crate::hook::StepOrGroup::Step(step) => {
                        validate_step(step, step_name, &format!("in hook '{hook_name}'"))?;
                    }
                    crate::hook::StepOrGroup::Group(group) => {
                        for (group_step_name, group_step) in &group.steps {
                            validate_step(
                                group_step,
                                group_step_name,
                                &format!("in group '{step_name}' of hook '{hook_name}'"),
                            )?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_step(step: &crate::step::Step, step_name: &str, location: &str) -> Result<()> {
    if step.stage.is_some() && step.fix.is_none() {
        bail!(
            "Step '{}' {} has 'stage' attribute but no 'fix' command. \
            Steps that stage files must have a fix command.",
            step_name,
            location
        );
    }

    let Some(selectors) = &step.match_any else {
        return Ok(());
    };

    if step.glob.is_some() || step.types.is_some() {
        bail!(
            "Step '{}' {} cannot combine 'match_any' with top-level 'glob' or 'types'.",
            step_name,
            location
        );
    }
    if selectors.is_empty() {
        bail!(
            "Step '{}' {} has an empty 'match_any'; add at least one selector.",
            step_name,
            location
        );
    }
    for (index, selector) in selectors.iter().enumerate() {
        if selector
            .glob
            .as_ref()
            .is_some_and(crate::step::Pattern::is_empty)
        {
            bail!(
                "Step '{}' {} has an empty 'glob' in 'match_any' selector {}.",
                step_name,
                location,
                index + 1
            );
        }
        if selector.types.as_ref().is_some_and(Vec::is_empty) {
            bail!(
                "Step '{}' {} has an empty 'types' in 'match_any' selector {}.",
                step_name,
                location,
                index + 1
            );
        }
        if selector.is_empty() {
            bail!(
                "Step '{}' {} has an empty 'match_any' selector {}. \
                Each selector must define a non-empty 'glob' or 'types'.",
                step_name,
                location,
                index + 1
            );
        }
    }

    Ok(())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImportAnalysis {
    local_paths: IndexSet<PathBuf>,
    has_untracked_imports: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::{Hook, StepOrGroup};
    use crate::step::Step;
    use crate::step_group::StepGroup;

    fn step(name: &str) -> Step {
        Step {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn hook(name: &str) -> Hook {
        Hook {
            name: name.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn top_level_steps_create_default_hooks_with_explicit_overrides() {
        let mut config = Config::default();
        config.steps.insert(
            "shared".to_string(),
            StepOrGroup::Step(Box::new(step("shared"))),
        );
        let mut check = hook("check");
        check.steps.insert(
            "shared".to_string(),
            StepOrGroup::Step(Box::new(Step {
                env: IndexMap::from([("SOURCE".to_string(), "explicit".to_string())]),
                ..Default::default()
            })),
        );
        config.hooks.insert("check".to_string(), check);

        config.materialize_default_hooks().unwrap();

        assert_eq!(
            config.hooks.keys().collect::<Vec<_>>(),
            ["check", "fix", "pre-commit"]
        );
        let check = config.hooks.get("check").unwrap();
        let StepOrGroup::Step(shared) = check.steps.get("shared").unwrap() else {
            panic!("expected step");
        };
        assert_eq!(
            shared.env.get("SOURCE").map(String::as_str),
            Some("explicit")
        );
        assert_eq!(check.fix, Some(false));
        assert_eq!(check.stage, Some(false));
        assert_eq!(config.hooks["fix"].fix, Some(true));
        assert_eq!(config.hooks["fix"].stage, Some(false));
        assert_eq!(config.hooks["pre-commit"].fix, Some(true));
        assert_eq!(config.hooks["pre-commit"].stage, Some(true));
        assert_eq!(
            config.hooks["pre-commit"].stash,
            Some(crate::hook::StashSetting::Method(
                crate::git::StashMethod::Git
            ))
        );
    }

    #[test]
    fn hkrc_top_level_steps_are_additive_and_project_wins() {
        let mut project = Config::default();
        project.steps.insert(
            "shared".to_string(),
            StepOrGroup::Step(Box::new(Step {
                env: IndexMap::from([("SOURCE".to_string(), "project".to_string())]),
                ..Default::default()
            })),
        );
        let mut user = Config::default();
        user.steps.insert(
            "shared".to_string(),
            StepOrGroup::Step(Box::new(step("user"))),
        );
        user.steps.insert(
            "user-only".to_string(),
            StepOrGroup::Step(Box::new(step("user-only"))),
        );
        let mut user_check = hook("check");
        user_check.steps.insert(
            "shared".to_string(),
            StepOrGroup::Step(Box::new(Step {
                env: IndexMap::from([("SOURCE".to_string(), "user-hook".to_string())]),
                ..Default::default()
            })),
        );
        user.hooks.insert("check".to_string(), user_check);

        project.materialize_default_hooks().unwrap();
        user.materialize_default_hooks().unwrap();
        project.merge_from_hkrc(user);

        let StepOrGroup::Step(shared) = &project.hooks["check"].steps["shared"] else {
            panic!("expected step");
        };
        assert_eq!(
            shared.env.get("SOURCE").map(String::as_str),
            Some("project")
        );
        assert!(project.hooks["check"].steps.contains_key("user-only"));
    }

    #[test]
    fn merge_subproject_scopes_flat_steps() {
        let mut root = Config::default();
        let mut sub = Config::default();
        sub.env.insert("FOO".to_string(), "from-config".to_string());

        let mut hook = hook("check");
        let lint = Step {
            depends: vec!["fmt".to_string(), "external".to_string()],
            ..step("lint")
        };
        let mut fmt = Step {
            dir: Some("nested".to_string()),
            ..step("fmt")
        };
        fmt.env.insert("FOO".to_string(), "from-step".to_string());
        hook.steps
            .insert("lint".to_string(), StepOrGroup::Step(Box::new(lint)));
        hook.steps
            .insert("fmt".to_string(), StepOrGroup::Step(Box::new(fmt)));
        sub.hooks.insert("check".to_string(), hook);

        root.merge_subproject("packages/web", sub).unwrap();

        let hook = root.hooks.get("check").unwrap();
        let StepOrGroup::Step(lint) = hook.steps.get("packages/web:lint").unwrap() else {
            panic!("expected step");
        };
        assert_eq!(lint.name, "packages/web:lint");
        assert_eq!(lint.dir.as_deref(), Some("packages/web"));
        // sibling references are rewritten; unknown names are left alone
        assert_eq!(
            lint.depends,
            vec!["packages/web:fmt".to_string(), "external".to_string()]
        );
        assert_eq!(lint.env.get("FOO").map(String::as_str), Some("from-config"));

        let StepOrGroup::Step(fmt) = hook.steps.get("packages/web:fmt").unwrap() else {
            panic!("expected step");
        };
        assert_eq!(fmt.dir.as_deref(), Some("packages/web/nested"));
        // step env wins over subproject config env
        assert_eq!(fmt.env.get("FOO").map(String::as_str), Some("from-step"));
    }

    #[test]
    fn merge_subproject_scopes_groups() {
        let mut root = Config::default();
        root.hooks.insert("check".to_string(), hook("check"));

        let mut sub = Config::default();
        let mut sub_hook = hook("check");
        let mut group = StepGroup {
            name: Some("build".to_string()),
            dir: Some("ui".to_string()),
            ..Default::default()
        };
        let ts = Step {
            dir: Some("ui".to_string()), // as propagated by group.init
            ..step("ts")
        };
        let tsc = Step {
            depends: vec!["ts".to_string()],
            ..step("tsc")
        };
        group.steps.insert("ts".to_string(), ts);
        group.steps.insert("tsc".to_string(), tsc);
        sub_hook
            .steps
            .insert("build".to_string(), StepOrGroup::Group(Box::new(group)));
        sub.hooks.insert("check".to_string(), sub_hook);

        root.merge_subproject("sub", sub).unwrap();

        let hook = root.hooks.get("check").unwrap();
        let StepOrGroup::Group(group) = hook.steps.get("sub:build").unwrap() else {
            panic!("expected group");
        };
        assert_eq!(group.name.as_deref(), Some("sub:build"));
        assert_eq!(group.dir.as_deref(), Some("sub/ui"));
        let ts = group.steps.get("ts").unwrap();
        assert_eq!(ts.name, "ts");
        assert_eq!(ts.dir.as_deref(), Some("sub/ui"));
        // Group child names are NOT prefixed, and hk builds a per-group
        // dependency tracker keyed by those child names, so intra-group
        // `depends` must stay unprefixed to keep resolving after merge.
        let tsc = group.steps.get("tsc").unwrap();
        assert_eq!(tsc.depends, vec!["ts".to_string()]);
    }

    #[test]
    fn merge_subproject_duplicate_name_errors() {
        let mut root = Config::default();
        let mut root_hook = hook("check");
        root_hook.steps.insert(
            "sub:lint".to_string(),
            StepOrGroup::Step(Box::new(step("sub:lint"))),
        );
        root.hooks.insert("check".to_string(), root_hook);

        let mut sub = Config::default();
        let mut sub_hook = hook("check");
        sub_hook.steps.insert(
            "lint".to_string(),
            StepOrGroup::Step(Box::new(step("lint"))),
        );
        sub.hooks.insert("check".to_string(), sub_hook);

        let err = root.merge_subproject("sub", sub).unwrap_err();
        assert!(err.to_string().contains("duplicate step name 'sub:lint'"));
    }

    #[test]
    fn merge_subproject_ignores_disabled_hooks() {
        let mut root = Config::default();
        let mut sub = Config::default();
        let mut disabled = hook("pre-commit");
        disabled.enabled = false;
        disabled.steps.insert(
            "lint".to_string(),
            StepOrGroup::Step(Box::new(step("lint"))),
        );
        sub.hooks.insert("pre-commit".to_string(), disabled);

        root.merge_subproject("sub", sub).unwrap();

        assert!(!root.hooks.contains_key("pre-commit"));
    }

    #[test]
    fn join_subdir_handles_nested_and_empty() {
        assert_eq!(Config::join_subdir("sub", None), "sub");
        assert_eq!(Config::join_subdir("sub", Some("")), "sub");
        assert_eq!(Config::join_subdir("sub", Some("ui")), "sub/ui");
    }

    #[test]
    fn project_root_of_handles_config_directory() {
        assert_eq!(
            Config::project_root_of(Path::new("/repo/hk.pkl")),
            PathBuf::from("/repo")
        );
        assert_eq!(
            Config::project_root_of(Path::new("/repo/.config/hk.pkl")),
            PathBuf::from("/repo")
        );
        assert_eq!(
            Config::project_root_of(Path::new("hk.pkl")),
            PathBuf::from(".")
        );
        assert_eq!(
            Config::project_root_of(Path::new(".config/hk.pkl")),
            PathBuf::from(".")
        );
    }

    #[test]
    fn discover_subprojects_literal_and_glob() {
        let base = std::env::temp_dir().join(format!(
            "hk-test-discover-subprojects-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        for dir in [
            "sub",
            "packages/a",
            "packages/b",
            "packages/.hidden",
            "node_modules/pkg",
        ] {
            std::fs::create_dir_all(base.join(dir)).unwrap();
        }
        for config in [
            "sub/hk.pkl",
            "packages/a/hk.pkl",
            "packages/.hidden/hk.pkl",
            "node_modules/pkg/hk.pkl",
        ] {
            std::fs::write(base.join(config), "").unwrap();
        }

        let found =
            Config::discover_subprojects(&base, &["sub".to_string(), "packages/*".to_string()])
                .unwrap();
        let dirs = found.iter().map(|(d, _)| d.as_str()).collect::<Vec<_>>();
        // packages/b has no config, hidden dirs and node_modules are skipped
        assert_eq!(dirs, vec!["sub", "packages/a"]);
        assert_eq!(found[0].1, base.join("sub/hk.pkl"));

        std::fs::remove_dir_all(&base).unwrap();
    }
}
