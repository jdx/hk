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
        config.load_subprojects()?;
        config.apply_hkrc()?;
        config.validate()?;
        Ok(config)
    }

    #[tracing::instrument(level = "info", name = "config.read", skip_all, fields(path = %path.display()))]
    fn read(path: &Path, apply_env: bool) -> Result<Self> {
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
                if env::use_pklr_backend() {
                    run_pklr(path)?
                } else {
                    run_pkl(&["eval"], path)?
                }
            }
            _ => {
                bail!("Unsupported file extension: {}", ext);
            }
        };
        config.init(path, apply_env)?;
        Ok(config)
    }

    /// Analyze pkl imports to get all transitive dependencies.
    /// Returns local file paths that the config depends on and whether the
    /// module graph contains imports whose bytes hk cannot hash.
    fn analyze_imports(path: &Path) -> Result<ImportAnalysis> {
        if env::use_pklr_backend() {
            let local_paths: IndexSet<PathBuf> = pklr::analyze_imports(path)
                .map(|v| v.into_iter().collect())
                .map_err(|e| eyre::eyre!("{e}"))?;
            let has_untracked_imports =
                Self::has_untracked_imports_in_pkl_sources(path, &local_paths)?;
            return Ok(ImportAnalysis {
                local_paths,
                has_untracked_imports,
            });
        }
        let imports: PklImports =
            run_pkl(&["analyze", "imports"], path).wrap_err("failed to analyze pkl")?;

        // Extract all local file paths from the imports map keys
        let mut local_paths = IndexSet::new();
        let mut has_untracked_imports = false;
        for uri in imports.resolvedImports.keys() {
            if let Some(file_path) = uri.strip_prefix("file://") {
                local_paths.insert(PathBuf::from(file_path));
            } else if Self::is_untracked_import_uri(uri) {
                has_untracked_imports = true;
            }
        }

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

    fn is_untracked_import_uri(uri: &str) -> bool {
        uri.starts_with("http://") || uri.starts_with("https://") || uri.starts_with("package://")
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
        let mut cwd = std::env::current_dir().ok()?;
        while cwd != Path::new("/") {
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

            // Always include the main config file. The pklr backend's
            // analyze_imports does not include the source file in its
            // output, so without this edits to hk.pkl wouldn't invalidate
            // the cache when using pklr. Using IndexSet avoids
            // double-listing the path on the pkl CLI backend, whose
            // resolvedImports already contains it.
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
        let config_cache_builder = CacheManagerBuilder::new(config_cache_path)
            .with_cache_key(format!("pkl-backend:{}", env::HK_PKL_BACKEND.as_str()));
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
                step.match_any = None;
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
            deprecated_at!(
                "1.37.0",
                "2.0.0",
                "hkrc-flag",
                "--hkrc is deprecated. Use {}/config.pkl for global config \
                 or hk.local.pkl for per-project overrides.",
                env::HK_CONFIG_DIR.display()
            );
            Some(path)
        } else {
            // Default discovery: CWD, then $HOME, then XDG config dir
            let cwd_path = PathBuf::from(".hkrc.pkl");
            let home_path = env::HOME_DIR.join(".hkrc.pkl");
            let xdg_path = env::HK_CONFIG_DIR.join("config.pkl");
            if cwd_path.exists() {
                deprecated_at!(
                    "1.37.0",
                    "2.0.0",
                    "hkrc-cwd",
                    ".hkrc.pkl is deprecated. Use hk.local.pkl in the project root instead."
                );
                Some(cwd_path)
            } else if home_path.exists() {
                deprecated_at!(
                    "1.37.0",
                    "2.0.0",
                    "hkrc-home",
                    "~/.hkrc.pkl is deprecated. Use {}/config.pkl instead.",
                    env::HK_CONFIG_DIR.display()
                );
                Some(home_path)
            } else if xdg_path.exists() {
                Some(xdg_path) // blessed path — no warning
            } else {
                None
            }
        };

        if let Some(path) = hkrc_path {
            // Parse pkl output as raw JSON for format detection
            let json_value: serde_json::Value = if env::use_pklr_backend() {
                run_pklr(&path)?
            } else {
                run_pkl(&["eval"], &path)?
            };

            // Backward compat: legacy hkrc files amend UserConfig.pkl (has "environment" key),
            // new-style hkrc files amend Config.pkl (has "env" key).
            if json_value.get("environment").is_some() {
                let user_config: UserConfig = serde_json::from_value(json_value)
                    .wrap_err("failed to parse hkrc as UserConfig")?;
                self.apply_user_config(&Some(user_config))?;
            } else {
                let mut hkrc_config: Config = serde_json::from_value(json_value)
                    .wrap_err("failed to parse hkrc as Config")?;
                hkrc_config.init(&path, true)?;
                self.merge_from_hkrc(hkrc_config);
            }
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
        let root = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
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
            let max_depth = glob_patterns
                .iter()
                .map(|p| {
                    if p.contains("**") {
                        usize::MAX
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
        if sub.subprojects.as_ref().is_some_and(|s| !s.is_empty()) {
            warn!(
                "subprojects: nested `subprojects` in {} is ignored (only one level is supported)",
                sub.path.display()
            );
        }
        let sub_env = std::mem::take(&mut sub.env);
        for (hook_name, sub_hook) in std::mem::take(&mut sub.hooks) {
            let root_hook = self.hooks.entry(hook_name.clone()).or_insert_with(|| Hook {
                name: hook_name.clone(),
                fix: sub_hook.fix,
                stash: sub_hook.stash.clone(),
                stage: sub_hook.stage,
                fail_on_fix: sub_hook.fail_on_fix,
                report: sub_hook.report.clone(),
                ..Default::default()
            });
            // Names of the subproject hook's flat steps, for rewriting `depends`
            // references to their scoped names.
            let flat_names = sub_hook
                .steps
                .iter()
                .filter(|(_, sog)| matches!(sog, crate::hook::StepOrGroup::Step(_)))
                .map(|(name, _)| name.clone())
                .collect::<IndexSet<_>>();
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
                                if flat_names.contains(dep) {
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

fn run_pklr<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let client = build_pklr_http_client()?;
    let http_rewrites = env::HK_PKL_HTTP_REWRITE
        .as_deref()
        .map(|s| s.split(',').map(String::from).collect::<Vec<_>>())
        .unwrap_or_default();
    let options = pklr::EvalOptions {
        client: Some(client),
        http_rewrites,
    };
    let rt = tokio::runtime::Handle::try_current();
    let json = match rt {
        Ok(handle) => tokio::task::block_in_place(|| {
            handle.block_on(pklr::eval_to_json_with_options(path, options))
        }),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(pklr::eval_to_json_with_options(path, options))
        }
    }
    .map_err(|e| handle_pklr_eval_error(&e.to_string(), path))?;
    serde_json::from_value(json).map_err(|e| handle_pklr_deserialize_error(&e.to_string(), path))
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

        if let Some(ca_certificates) = env::HK_PKL_CA_CERTIFICATES.as_ref() {
            args.push("--ca-certificates".to_string());
            args.push(ca_certificates.display().to_string());
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
        return Err(missing_amends_error(path));
    } else if stderr.contains("Module URI") && stderr.contains("has invalid syntax") {
        return Err(invalid_module_uri_error(path));
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

fn handle_pklr_eval_error(error: &str, path: &Path) -> eyre::Report {
    if error.contains("unsupported package URI")
        || (error.contains("Module URI") && error.contains("has invalid syntax"))
    {
        return invalid_module_uri_error(path);
    }
    failed_pkl_config_error(path, None, error)
}

fn handle_pklr_deserialize_error(error: &str, path: &Path) -> eyre::Report {
    if !pkl_file_has_amends(path) && error.contains("unknown field") {
        return missing_amends_error(path);
    }
    eyre::eyre!("failed to deserialize pklr output\n\nCaused by:\n    {error}")
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

fn failed_pkl_config_error(path: &Path, code: Option<&str>, stderr: &str) -> eyre::Report {
    match code {
        Some(code) => eyre::eyre!(
            "Failed to evaluate Pkl config at {}\n\nExit code: {}\n\nError output:\n{}",
            path.display(),
            code,
            stderr
        ),
        None => eyre::eyre!(
            "Failed to evaluate Pkl config at {}\n\nError output:\n{}",
            path.display(),
            stderr
        ),
    }
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
        group.steps.insert("ts".to_string(), ts);
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
        // group child names are not prefixed; in-group depends still work
        assert_eq!(ts.name, "ts");
        assert_eq!(ts.dir.as_deref(), Some("sub/ui"));
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
    fn join_subdir_handles_nested_and_empty() {
        assert_eq!(Config::join_subdir("sub", None), "sub");
        assert_eq!(Config::join_subdir("sub", Some("")), "sub");
        assert_eq!(Config::join_subdir("sub", Some("ui")), "sub/ui");
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
