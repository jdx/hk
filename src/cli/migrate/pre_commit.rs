use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::Result;
use eyre::bail;
use indexmap::IndexMap;
use serde::Deserialize;
use shell_quote::Quote;

use super::{HkConfig, HkHook, HkStep};

/// Migrate from pre-commit to hk
#[derive(Debug, clap::Args)]
pub struct PreCommit {
    /// Path to .pre-commit-config.yaml
    #[clap(short, long, default_value = ".pre-commit-config.yaml")]
    config: PathBuf,
    /// Output path for hk.pkl
    #[clap(short, long, default_value = "hk.pkl")]
    output: PathBuf,
    /// Overwrite existing hk.pkl file
    #[clap(short, long)]
    force: bool,
    /// Root path for hk pkl files (e.g., "pkl" for local, or package URL prefix)
    /// If specified, will use {root}/Config.pkl and {root}/Builtins.pkl
    #[clap(long)]
    hk_pkl_root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PreCommitConfig {
    repos: Vec<PreCommitRepo>,
    #[serde(default)]
    fail_fast: bool,
    #[serde(default)]
    default_language_version: HashMap<String, String>,
    #[serde(default)]
    default_stages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PreCommitRepo {
    repo: String,
    #[serde(default)]
    rev: Option<String>,
    hooks: Vec<PreCommitHook>,
}

#[derive(Debug, Deserialize)]
struct PreCommitHook {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    entry: Option<String>,
    #[serde(default)]
    files: Option<String>,
    #[serde(default)]
    exclude: Option<String>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    types_or: Vec<String>,
    #[serde(default)]
    exclude_types: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    additional_dependencies: Vec<String>,
    #[serde(default)]
    stages: Vec<String>,
    #[serde(default)]
    always_run: bool,
    #[serde(default)]
    pass_filenames: Option<bool>,
    #[serde(default)]
    language_version: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    require_serial: bool,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PreCommitHookDefinition {
    id: String,
    #[serde(default)]
    #[allow(dead_code)]
    name: Option<String>,
    #[serde(default)]
    entry: String,
    #[serde(default)]
    language: String,
    #[serde(default)]
    files: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    types: Vec<String>,
    #[serde(default)]
    pass_filenames: Option<bool>,
}

struct VendoredRepo {
    url: String,
    name: String,
    #[allow(dead_code)]
    vendor_path: PathBuf,
    hooks: Vec<PreCommitHookDefinition>,
}

impl PreCommit {
    pub async fn run(&self) -> Result<()> {
        if self.output.exists() && !self.force {
            bail!(
                "{} already exists, use --force to overwrite",
                self.output.display()
            );
        }

        if !self.config.exists() {
            bail!("{} does not exist", self.config.display());
        }

        let config_content = xx::file::read_to_string(&self.config)?;
        let precommit_config: PreCommitConfig = serde_yaml::from_str(&config_content)?;

        // Vendor external repos
        let vendored_repos = self.vendor_repos(&precommit_config).await?;

        let (amends_config_pkl, types_pkl_import, builtins_pkl) =
            if let Some(ref root) = self.hk_pkl_root {
                (
                    Some(format!("{}/Config.pkl", root)),
                    Some(format!("{}/Types.pkl", root)),
                    Some(format!("{}/Builtins.pkl", root)),
                )
            } else {
                (None, None, None)
            };

        let hk_config = self.convert_config(
            &precommit_config,
            &vendored_repos,
            amends_config_pkl,
            types_pkl_import,
            builtins_pkl,
        )?;
        let pkl_content = hk_config.to_pkl();

        xx::file::write(&self.output, pkl_content)?;

        info!(
            "Migrated {} to {}",
            self.config.display(),
            self.output.display()
        );
        println!("Successfully migrated to hk.pkl!");

        println!("\nNext steps:");
        println!("1. Review the generated hk.pkl file");
        println!("2. Complete any TODO items (local/unknown hooks, vendored repos)");
        println!("3. Run 'hk install' to install git hooks");
        println!("4. Run 'hk check --all' to test your configuration");

        Ok(())
    }

    fn convert_config(
        &self,
        config: &PreCommitConfig,
        vendored_repos: &HashMap<String, VendoredRepo>,
        amends_config_pkl: Option<String>,
        types_pkl_import: Option<String>,
        builtins_pkl: Option<String>,
    ) -> Result<HkConfig> {
        let mut hk_config = HkConfig::new(amends_config_pkl, types_pkl_import, builtins_pkl);
        let mut used_ids = HashSet::new();

        // Add imports for vendored repos
        for vendor in vendored_repos.values() {
            let import_name = Self::repo_url_to_import_name(&vendor.url);
            let import_path = format!(".hk/vendors/{}/hooks.pkl", vendor.name);
            hk_config.vendor_imports.push((import_name, import_path));
        }

        // Add header comments
        if !vendored_repos.is_empty() {
            hk_config
                .header_comments
                .push("TODO: Vendored repos detected".to_string());
            hk_config.header_comments.push(
                "The .hk/vendors directory is a compatibility layer for pre-commit projects."
                    .to_string(),
            );
            hk_config
                .header_comments
                .push("For better performance and idiomatic hk usage, consider installing tools with mise:".to_string());
            hk_config
                .header_comments
                .push("  mise use <tool>@<version>".to_string());
            hk_config.header_comments.push(
                "Then update your hooks to use the mise-installed tools directly.".to_string(),
            );
            hk_config.header_comments.push("".to_string());
        }
        if config.fail_fast {
            hk_config
                .header_comments
                .push("Migrated from pre-commit fail_fast setting".to_string());
            hk_config
                .header_comments
                .push("Note: hk uses --fail-fast CLI flag instead of config setting".to_string());
            hk_config.header_comments.push("".to_string());
        }

        if !config.default_language_version.is_empty() {
            hk_config
                .header_comments
                .push("pre-commit default_language_version:".to_string());

            // Warn user about language versions
            warn!("Detected default_language_version in pre-commit config");
            println!("\nLanguage versions detected in .pre-commit-config.yaml:");

            for (lang, version) in &config.default_language_version {
                hk_config
                    .header_comments
                    .push(format!("  {}: {}", lang, version));

                let normalized_version = Self::normalize_language_version(version);

                // Print warning with mise use command
                println!(
                    "  {}: {} -> Run: mise use {}@{}",
                    lang, version, lang, normalized_version
                );
            }

            hk_config.header_comments.push("".to_string());
            hk_config
                .header_comments
                .push("To set these versions with mise, run:".to_string());

            for (lang, version) in &config.default_language_version {
                let normalized_version = Self::normalize_language_version(version);
                hk_config
                    .header_comments
                    .push(format!("  mise use {}@{}", lang, normalized_version));
            }
        }

        // Initialize step collections
        let mut linters = IndexMap::new();
        let mut local_hooks = IndexMap::new();
        let mut custom_steps = IndexMap::new();
        let mut manual_steps = IndexMap::new();

        // Track which stages each hook appears in
        let mut steps_by_stage: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for repo in &config.repos {
            let is_local = repo.repo == "local";
            let is_meta = repo.repo == "meta";

            if is_meta {
                // Skip meta hooks, they're pre-commit internal
                continue;
            }

            for hook in &repo.hooks {
                let unique_id = Self::make_unique_hook_id(&hook.id, &mut used_ids);

                let hook_stages = if !hook.stages.is_empty() {
                    hook.stages.clone()
                } else if !config.default_stages.is_empty() {
                    config.default_stages.clone()
                } else {
                    vec!["pre-commit".to_string()]
                };

                // Check if this is a manual-only step
                let is_manual_only = hook_stages.len() == 1 && hook_stages[0] == "manual";

                // Convert the hook to an HkStep
                let step = if is_local {
                    self.convert_local_hook(hook, &unique_id, repo)
                } else if let Some(step) = self.convert_known_hook(hook, &unique_id) {
                    step
                } else {
                    self.convert_unknown_hook(hook, &unique_id, repo, vendored_repos)
                };

                // Add to appropriate collection
                let is_vendored = vendored_repos.contains_key(&repo.repo);
                let collection_name = if is_manual_only {
                    manual_steps.insert(unique_id.clone(), step);
                    "manual_steps"
                } else if is_local {
                    local_hooks.insert(unique_id.clone(), step);
                    "local_hooks"
                } else if self.is_known_hook(&hook.id) || is_vendored {
                    linters.insert(unique_id.clone(), step);
                    "linters"
                } else {
                    custom_steps.insert(unique_id.clone(), step);
                    "custom_steps"
                };

                // Track which stages this hook appears in
                for stage in &hook_stages {
                    let hk_stage = Self::map_stage(stage);
                    steps_by_stage
                        .entry(hk_stage.to_string())
                        .or_default()
                        .push((unique_id.clone(), collection_name.to_string()));
                }
            }
        }

        // Add step collections to config
        if !linters.is_empty() {
            hk_config
                .step_collections
                .insert("linters".to_string(), linters);
        }
        if !local_hooks.is_empty() {
            hk_config
                .step_collections
                .insert("local_hooks".to_string(), local_hooks);
        }
        if !custom_steps.is_empty() {
            hk_config
                .step_collections
                .insert("custom_steps".to_string(), custom_steps);
        }
        if !manual_steps.is_empty() {
            hk_config
                .step_collections
                .insert("manual_steps".to_string(), manual_steps);
        }

        // Generate hooks
        let has_steps = !hk_config.step_collections.is_empty();

        // Create hooks for each stage (except manual, which goes to check/fix)
        let mut stages_used = HashSet::new();
        for stage in steps_by_stage.keys() {
            // Skip manual stage - those steps will be added to check/fix hooks
            if stage == "manual" {
                continue;
            }

            stages_used.insert(stage.clone());

            let mut hook = HkHook {
                fix: None,
                stash: None,
                step_spreads: Vec::new(),
                direct_steps: IndexMap::new(),
            };

            if stage == "pre-commit" {
                hook.fix = Some(true);
                hook.stash = Some("git".to_string());
            }

            // Add step spreads for collections that have steps in this stage
            let collections_in_stage: HashSet<String> = steps_by_stage
                .get(stage)
                .map(|steps| steps.iter().map(|(_, col)| col.clone()).collect())
                .unwrap_or_default();

            // Only include non-manual collections in git hooks
            for collection in ["linters", "local_hooks", "custom_steps"] {
                if collections_in_stage.contains(collection) {
                    hook.step_spreads.push(collection.to_string());
                }
            }

            hk_config.hooks.insert(stage.clone(), hook);
        }

        // Always add check and fix hooks if we have any steps
        if has_steps {
            // Check hook - includes manual_steps
            let mut check_hook = HkHook {
                fix: None,
                stash: None,
                step_spreads: Vec::new(),
                direct_steps: IndexMap::new(),
            };

            for collection in &["linters", "local_hooks", "custom_steps", "manual_steps"] {
                if hk_config.step_collections.contains_key(*collection) {
                    check_hook.step_spreads.push(collection.to_string());
                }
            }

            hk_config.hooks.insert("check".to_string(), check_hook);

            // Fix hook - includes manual_steps and custom_steps
            let mut fix_hook = HkHook {
                fix: Some(true),
                stash: None,
                step_spreads: Vec::new(),
                direct_steps: IndexMap::new(),
            };

            for collection in &["linters", "local_hooks", "custom_steps", "manual_steps"] {
                if hk_config.step_collections.contains_key(*collection) {
                    fix_hook.step_spreads.push(collection.to_string());
                }
            }

            hk_config.hooks.insert("fix".to_string(), fix_hook);
        }

        Ok(hk_config)
    }

    fn convert_known_hook(&self, hook: &PreCommitHook, unique_id: &str) -> Option<HkStep> {
        let builtin_map = self.get_builtin_map();

        if let Some(builtin_name) = builtin_map.get(hook.id.as_str()) {
            let mut step = HkStep {
                builtin: Some(format!("Builtins.{}", builtin_name)),
                comments: Vec::new(),
                glob: None,
                exclude: Self::add_default_exclude(hook.exclude.clone()),
                prefix: None,
                check: None,
                fix: None,
                shell: None,
                exclusive: hook.require_serial,
                properties_as_comments: Vec::new(),
            };

            // Add comments if ID was changed
            if unique_id != hook.id {
                step.comments.push(format!("Original ID: {}", hook.id));
            }

            // Add property comments for things we can't directly map
            if hook.files.is_some() {
                if let Some(ref files) = hook.files {
                    step.properties_as_comments
                        .push(format!("files pattern from pre-commit: {}", files));
                }
                step.properties_as_comments
                    .push("Note: Convert regex to glob pattern for hk".to_string());
            }

            if !hook.types.is_empty() {
                step.properties_as_comments
                    .push(format!("types (AND): {}", hook.types.join(", ")));
            }

            if !hook.types_or.is_empty() {
                step.properties_as_comments
                    .push(format!("types_or: {}", hook.types_or.join(", ")));
            }

            if !hook.exclude_types.is_empty() {
                step.properties_as_comments
                    .push(format!("exclude_types: {}", hook.exclude_types.join(", ")));
            }

            if hook.always_run {
                step.properties_as_comments
                    .push("always_run: true - runs even without matching files".to_string());
                step.properties_as_comments.push(
                    "Note: hk doesn't have direct equivalent, hook will run on all files"
                        .to_string(),
                );
            }

            if hook.pass_filenames == Some(false) {
                step.properties_as_comments
                    .push("pass_filenames: false".to_string());
                step.properties_as_comments
                    .push("Note: Adjust check/fix commands to not use {{files}}".to_string());
            }

            if !hook.args.is_empty() {
                step.properties_as_comments
                    .push(format!("args from pre-commit: {}", hook.args.join(" ")));
                step.properties_as_comments
                    .push("Consider updating check/fix commands with these args".to_string());
            }

            if !hook.additional_dependencies.is_empty() {
                step.properties_as_comments.push(format!(
                    "additional_dependencies: {}",
                    hook.additional_dependencies.join(", ")
                ));
                step.properties_as_comments
                    .push("Use mise x to install dependencies:".to_string());
                let tool_name = self.hook_id_to_tool(&hook.id);
                step.properties_as_comments
                    .push(format!("prefix = \"mise x {}@latest --\"", tool_name));
            }

            if let Some(ref lang_ver) = hook.language_version {
                step.properties_as_comments
                    .push(format!("language_version: {}", lang_ver));
                step.properties_as_comments
                    .push("Configure version in mise.toml".to_string());
            }

            Some(step)
        } else {
            None
        }
    }

    fn convert_local_hook(
        &self,
        hook: &PreCommitHook,
        unique_id: &str,
        _repo: &PreCommitRepo,
    ) -> HkStep {
        let mut step = HkStep {
            builtin: None,
            comments: Vec::new(),
            glob: hook.files.clone(),
            exclude: Self::add_default_exclude(hook.exclude.clone()),
            prefix: None,
            check: None,
            fix: None,
            shell: None,
            exclusive: hook.require_serial,
            properties_as_comments: Vec::new(),
        };

        // Apply types/types_or filtering
        // If there's no glob pattern but we have types, create a glob from types
        // Otherwise, create exclude patterns for non-matching types
        if !hook.types.is_empty() || !hook.types_or.is_empty() {
            if step.glob.is_none() {
                // No files pattern - create a glob from types
                if let Some(glob_pattern) = Self::types_to_glob_pattern(&hook.types, &hook.types_or)
                {
                    step.glob = Some(glob_pattern);
                }
            } else {
                // Has files pattern - add exclude patterns for non-matching types
                if let Some(types_exclude) =
                    Self::types_to_exclude_pattern(&hook.types, &hook.types_or, &hook.exclude_types)
                {
                    // Combine with existing exclude pattern
                    if let Some(ref existing_exclude) = step.exclude {
                        step.exclude = Some(format!("{}|{}", existing_exclude, types_exclude));
                    } else {
                        step.exclude = Some(types_exclude);
                    }
                }
            }
        }

        // Add comments
        if unique_id != hook.id {
            step.comments.push(format!("Original ID: {}", hook.id));
        }
        if let Some(ref name) = hook.name {
            step.comments.push(format!("Name: {}", name));
        }

        // Add comment about type filtering if present
        if !hook.types.is_empty() {
            step.properties_as_comments
                .push(format!("types (AND): {}", hook.types.join(", ")));
        }
        if !hook.types_or.is_empty() {
            step.properties_as_comments
                .push(format!("types (OR): {}", hook.types_or.join(", ")));
        }

        // Handle additional_dependencies with mise x
        if !hook.additional_dependencies.is_empty() {
            step.prefix = Self::generate_mise_prefix(&hook.additional_dependencies);
        }

        // Set check command
        if let Some(ref entry) = hook.entry {
            let pass_filenames = hook.pass_filenames.unwrap_or(true);

            // Check if this is a pygrep hook - convert to grep command
            let is_pygrep = hook.language.as_deref() == Some("pygrep");

            // Check if this is a docker_image hook - convert to docker run command
            let is_docker_image = hook.language.as_deref() == Some("docker_image");

            // Check if this is a Python script that should use uv run
            // For multi-line entries, check if the first line/word ends with .py
            let is_python_script = hook.language.as_deref() == Some("python")
                && (entry.ends_with(".py")
                    || entry
                        .split_whitespace()
                        .next()
                        .is_some_and(|s| s.ends_with(".py")));

            let cmd = if is_pygrep {
                // pygrep hooks use the entry as a regex pattern
                // pre-commit's pygrep is a simple Python regex grep that works on any file
                // It returns 1 on match (problem found), 0 on no match (success)
                // We use grep -P for Perl-compatible regex (similar to Python regex)
                // and invert with ! so that finding a match returns an error
                let quoted_pattern: String = shell_quote::Bash::quote(entry);
                if pass_filenames {
                    format!("! grep -P {} {{{{files}}}}", quoted_pattern)
                } else {
                    format!("! grep -P {}", quoted_pattern)
                }
            } else if is_docker_image {
                // docker_image hooks use the entry as: <image_name> <args...>
                // Example: koalaman/shellcheck:v0.8.0 -x -a
                // We convert to: docker run --rm -v $(pwd):/src -w /src <image_name> <args...> {{files}}
                // The image name is the first token, the rest are arguments
                let parts: Vec<&str> = entry.split_whitespace().collect();
                let image_name = parts.first().unwrap_or(&"");
                let docker_args = parts[1..].join(" ");

                if pass_filenames {
                    if docker_args.is_empty() {
                        format!(
                            "docker run --rm -v $(pwd):/src -w /src {} {{{{files}}}}",
                            image_name
                        )
                    } else {
                        format!(
                            "docker run --rm -v $(pwd):/src -w /src {} {} {{{{files}}}}",
                            image_name, docker_args
                        )
                    }
                } else if docker_args.is_empty() {
                    format!("docker run --rm -v $(pwd):/src -w /src {}", image_name)
                } else {
                    format!(
                        "docker run --rm -v $(pwd):/src -w /src {} {}",
                        image_name, docker_args
                    )
                }
            } else if is_python_script {
                // Use uv run for local Python scripts
                if pass_filenames {
                    format!("uv run {} {{{{files}}}}", entry)
                } else {
                    format!("uv run {}", entry)
                }
            } else {
                // Use entry directly for non-Python scripts
                if pass_filenames {
                    format!("{} {{{{files}}}}", entry)
                } else {
                    entry.clone()
                }
            };

            // Add args to the command if present
            let final_cmd = if !hook.args.is_empty() {
                let args_str = hook.args.join(" ");
                // Insert args between the command and {{files}}
                if pass_filenames {
                    cmd.replace(" {{files}}", &format!(" {} {{{{files}}}}", args_str))
                } else {
                    format!("{} {}", cmd, args_str)
                }
            } else {
                cmd
            };

            step.check = Some(final_cmd);
            if !pass_filenames {
                step.properties_as_comments
                    .push("pass_filenames was false in pre-commit".to_string());
            }
        } else {
            step.properties_as_comments
                .push("TODO: Configure check and/or fix commands from local hook".to_string());
            step.properties_as_comments
                .push("check = \"...\"".to_string());

            if !hook.args.is_empty() {
                step.properties_as_comments
                    .push(format!("Original args: {}", hook.args.join(" ")));
            }
        }

        if hook.always_run {
            step.properties_as_comments
                .push("always_run was true in pre-commit".to_string());
        }

        step
    }

    fn convert_unknown_hook(
        &self,
        hook: &PreCommitHook,
        unique_id: &str,
        repo: &PreCommitRepo,
        vendored_repos: &HashMap<String, VendoredRepo>,
    ) -> HkStep {
        // Check if this hook is from a vendored repo
        if let Some(vendored) = vendored_repos.get(&repo.repo) {
            // Find the hook definition in the vendored repo
            if let Some(_hook_def) = vendored.hooks.iter().find(|h| h.id == hook.id) {
                let import_name = Self::repo_url_to_import_name(&repo.repo);
                let hook_id_snake = hook.id.replace('-', "_");

                let mut step = HkStep {
                    builtin: Some(format!("{}.{}", import_name, hook_id_snake)),
                    comments: Vec::new(),
                    glob: None,
                    exclude: Self::add_default_exclude(hook.exclude.clone()),
                    prefix: None,
                    check: None,
                    fix: None,
                    shell: None,
                    exclusive: hook.require_serial,
                    properties_as_comments: Vec::new(),
                };

                // Add comment if ID was changed
                if unique_id != hook.id {
                    step.comments.push(format!("Original ID: {}", hook.id));
                }

                // Add property comments for things we can't directly map
                if hook.files.is_some() {
                    if let Some(ref files) = hook.files {
                        step.properties_as_comments
                            .push(format!("files pattern from pre-commit: {}", files));
                    }
                }

                if !hook.types.is_empty() {
                    step.properties_as_comments
                        .push(format!("types (AND): {}", hook.types.join(", ")));
                }

                if !hook.args.is_empty() {
                    step.properties_as_comments
                        .push(format!("args from pre-commit: {}", hook.args.join(" ")));
                }

                return step;
            }
        }

        // Fallback to unknown hook generation
        let mut step = HkStep {
            builtin: None,
            comments: Vec::new(),
            glob: None,
            exclude: Self::add_default_exclude(hook.exclude.clone()),
            prefix: None,
            check: None,
            fix: None,
            shell: None,
            exclusive: hook.require_serial,
            properties_as_comments: Vec::new(),
        };

        // Add repo info as comment
        step.comments.push(format!(
            "Repo: {}{}",
            repo.repo,
            repo.rev
                .as_ref()
                .map_or(String::new(), |r| format!(" @ {}", r))
        ));

        if unique_id != hook.id {
            step.comments.push(format!("Original ID: {}", hook.id));
        }

        if let Some(ref name) = hook.name {
            step.comments.push(format!("Name: {}", name));
        }

        if let Some(ref files) = hook.files {
            step.properties_as_comments
                .push(format!("files: {}", files));
        }

        if !hook.types.is_empty() || !hook.types_or.is_empty() {
            step.properties_as_comments
                .push("File type filtering needed".to_string());
        }

        step.properties_as_comments
            .push("TODO: Configure check and/or fix commands".to_string());
        step.properties_as_comments
            .push("check = \"...\"".to_string());
        step.properties_as_comments
            .push("fix = \"...\"".to_string());

        if !hook.args.is_empty() {
            step.properties_as_comments
                .push(format!("Original args: {}", hook.args.join(" ")));
        }

        if !hook.additional_dependencies.is_empty() {
            step.properties_as_comments.push(format!(
                "Dependencies: {}",
                hook.additional_dependencies.join(", ")
            ));
        }

        step
    }

    fn is_known_hook(&self, id: &str) -> bool {
        self.get_builtin_map().contains_key(id)
    }

    /// Normalize language version strings for mise
    /// Example: "python3" -> "3", "3.11" -> "3.11"
    fn normalize_language_version(version: &str) -> String {
        // Handle common pre-commit language version formats
        match version {
            "python3" => "3".to_string(),
            "python2" => "2".to_string(),
            v if v.starts_with("python") => v.strip_prefix("python").unwrap_or(v).to_string(),
            v => v.to_string(),
        }
    }

    /// Generate a mise x prefix from additional_dependencies
    /// Example: ["ruff==0.13.3"] -> Some("mise x pipx:ruff@0.13.3 --")
    fn generate_mise_prefix(dependencies: &[String]) -> Option<String> {
        if dependencies.is_empty() {
            return None;
        }

        // For now, handle the first dependency (most common case)
        // Format: package==version or package>=version, etc.
        let dep = &dependencies[0];

        // Parse package name and version
        let (package, version) = if let Some(idx) = dep.find("==") {
            (&dep[..idx], Some(&dep[idx + 2..]))
        } else if let Some(idx) = dep.find(">=") {
            (&dep[..idx], Some(&dep[idx + 2..]))
        } else if let Some(idx) = dep.find("<=") {
            (&dep[..idx], Some(&dep[idx + 2..]))
        } else if let Some(idx) = dep.find('>') {
            (&dep[..idx], Some(&dep[idx + 1..]))
        } else if let Some(idx) = dep.find('<') {
            (&dep[..idx], Some(&dep[idx + 1..]))
        } else {
            (dep.as_str(), None)
        };

        // Build mise x command
        if let Some(ver) = version {
            Some(format!("mise x pipx:{}@{} --", package, ver))
        } else {
            Some(format!("mise x pipx:{} --", package))
        }
    }

    fn map_stage(stage: &str) -> &'static str {
        match stage {
            "commit" | "commit-msg" => "commit-msg",
            "push" | "pre-push" => "pre-push",
            "prepare-commit-msg" => "prepare-commit-msg",
            "manual" => "manual",
            _ => "pre-commit",
        }
    }

    /// Convert pre-commit types/types_or to a glob pattern
    /// This is used when there's no files pattern but we have types
    fn types_to_glob_pattern(types: &[String], types_or: &[String]) -> Option<String> {
        let match_types = if !types_or.is_empty() {
            types_or
        } else if !types.is_empty() {
            types
        } else {
            return None;
        };

        // Map types to glob patterns
        let mut patterns = Vec::new();
        for type_name in match_types {
            match type_name.as_str() {
                "python" => patterns.push("**/*.py"),
                "pyi" => patterns.push("**/*.pyi"),
                "yaml" => {
                    patterns.push("**/*.yaml");
                    patterns.push("**/*.yml");
                }
                "json" => patterns.push("**/*.json"),
                "toml" => patterns.push("**/*.toml"),
                "markdown" => {
                    patterns.push("**/*.md");
                    patterns.push("**/*.markdown");
                    patterns.push("**/*.mdown");
                }
                "javascript" => patterns.push("**/*.js"),
                "jsx" => patterns.push("**/*.jsx"),
                "typescript" => patterns.push("**/*.ts"),
                "tsx" => patterns.push("**/*.tsx"),
                "rust" => patterns.push("**/*.rs"),
                "go" => patterns.push("**/*.go"),
                "shell" => {
                    patterns.push("**/*.sh");
                    patterns.push("**/*.bash");
                }
                "text" | "file" => return None, // Match all files, no pattern needed
                _ => return None,               // Unknown type
            }
        }

        if patterns.is_empty() {
            return None;
        }

        // For types_or with multiple patterns, use regex alternation
        if patterns.len() == 1 {
            Some(patterns[0].to_string())
        } else {
            // Convert glob patterns to regex
            let regex_patterns: Vec<String> = patterns
                .iter()
                .map(|p| {
                    // Convert **/*.ext to regex pattern that matches end of filename
                    let ext = p.strip_prefix("**/").unwrap_or(p);
                    let pattern = ext.replace("*.", r".*\.");
                    format!("{}$", pattern) // Anchor to end of filename
                })
                .collect();
            Some(format!("({})", regex_patterns.join("|")))
        }
    }

    /// Convert pre-commit types/types_or to exclude patterns
    /// This creates a negative pattern to exclude files that don't match the specified types
    fn types_to_exclude_pattern(
        types: &[String],
        types_or: &[String],
        _exclude_types: &[String],
    ) -> Option<String> {
        // Build the list of types to match
        let mut match_types = Vec::new();

        if !types_or.is_empty() {
            // types_or: match any of these types
            match_types.extend(types_or.iter().cloned());
        } else if !types.is_empty() {
            // types: must match all of these (we'll use AND logic)
            match_types.extend(types.iter().cloned());
        } else {
            // No type filtering
            return None;
        }

        // Map common pre-commit types to file extensions
        let mut extensions = Vec::new();
        for type_name in &match_types {
            match type_name.as_str() {
                "python" => extensions.push("py"),
                "pyi" => extensions.push("pyi"),
                "yaml" => {
                    extensions.push("yaml");
                    extensions.push("yml");
                }
                "json" => extensions.push("json"),
                "toml" => extensions.push("toml"),
                "markdown" => {
                    extensions.push("md");
                    extensions.push("markdown");
                    extensions.push("mdown");
                }
                "javascript" => extensions.push("js"),
                "jsx" => extensions.push("jsx"),
                "typescript" => extensions.push("ts"),
                "tsx" => extensions.push("tsx"),
                "rust" => extensions.push("rs"),
                "go" => extensions.push("go"),
                "shell" => {
                    extensions.push("sh");
                    extensions.push("bash");
                }
                "text" => return None, // text matches everything, no exclude needed
                _ => {
                    // Unknown type, can't filter
                    return None;
                }
            }
        }

        if extensions.is_empty() {
            return None;
        }

        // Create a regex pattern that matches files we want to EXCLUDE
        // Since regex doesn't support lookahead, we'll list common file extensions to EXCLUDE
        // that are NOT in our allowed list

        // Common file extensions that might be in a repo
        let all_common_extensions = vec![
            "py", "pyi", "js", "jsx", "ts", "tsx", "json", "yaml", "yml", "toml", "md", "markdown",
            "mdown", "txt", "rst", "xml", "html", "css", "scss", "sh", "bash", "zsh", "fish", "c",
            "cpp", "h", "hpp", "rs", "go", "java", "kt", "rb", "php", "pl", "lua", "r", "sql",
            "proto", "graphql", "vue", "svelte",
        ];

        // Filter out extensions that are in our allowed list
        let excluded_extensions: Vec<&&str> = all_common_extensions
            .iter()
            .filter(|ext| !extensions.contains(&ext.to_string().as_str()))
            .collect();

        if excluded_extensions.is_empty() {
            // If we'd exclude nothing, don't add an exclude pattern
            return None;
        }

        // Build pattern to match files with extensions NOT in our list
        let mut patterns: Vec<String> = excluded_extensions
            .iter()
            .map(|ext| format!(r"\.{}$", ext))
            .collect();

        // Add common lock/config files that don't have standard extensions
        // These are typically not source code files
        let special_files = vec![
            r"uv\.lock$",
            r"Cargo\.lock$",
            r"package-lock\.json$",
            r"yarn\.lock$",
            r"pnpm-lock\.yaml$",
            r"poetry\.lock$",
            r"Gemfile\.lock$",
            r"Pipfile\.lock$",
        ];

        // Only add special files if they would be excluded (not in our allowed extensions)
        for special in special_files {
            patterns.push(special.to_string());
        }

        let exclude_pattern = patterns.join("|");

        Some(exclude_pattern)
    }

    /// Ensure hook IDs are unique by adding suffixes for duplicates
    fn make_unique_hook_id(id: &str, existing_ids: &mut HashSet<String>) -> String {
        if !existing_ids.contains(id) {
            existing_ids.insert(id.to_string());
            return id.to_string();
        }

        // Find a unique suffix
        let mut counter = 2;
        loop {
            let unique_id = format!("{}-{}", id, counter);
            if !existing_ids.contains(&unique_id) {
                existing_ids.insert(unique_id.clone());
                return unique_id;
            }
            counter += 1;
        }
    }

    /// Add default exclude pattern (.hk/) to an existing exclude pattern
    fn add_default_exclude(existing_exclude: Option<String>) -> Option<String> {
        const DEFAULT_EXCLUDE: &str = r"^\.hk/";

        match existing_exclude {
            Some(existing) if !existing.is_empty() => {
                Some(format!("{}|{}", existing, DEFAULT_EXCLUDE))
            }
            _ => Some(DEFAULT_EXCLUDE.to_string()),
        }
    }

    fn hook_id_to_tool(&self, hook_id: &str) -> String {
        match hook_id {
            "black" | "flake8" | "isort" | "mypy" | "pylint" => hook_id.to_string(),
            "ruff" => "ruff".to_string(),
            "prettier" | "eslint" => hook_id.to_string(),
            "rustfmt" | "cargo-fmt" => "rust".to_string(),
            "clippy" => "rust".to_string(),
            "shellcheck" | "shfmt" => hook_id.to_string(),
            "rubocop" => "ruby".to_string(),
            "gofmt" | "goimports" | "golangci-lint" | "go-vet" => "go".to_string(),
            "yamllint" => "yamllint".to_string(),
            "hadolint" => "hadolint".to_string(),
            "terraform-fmt" | "tflint" => "terraform".to_string(),
            "stylelint" => "node".to_string(),
            "markdownlint" => "node".to_string(),
            "actionlint" => "actionlint".to_string(),
            _ => hook_id.to_string(),
        }
    }

    fn get_builtin_map(&self) -> HashMap<&'static str, &'static str> {
        let mut map = HashMap::new();

        // Python
        map.insert("black", "black");
        map.insert("flake8", "flake8");
        map.insert("isort", "isort");
        map.insert("mypy", "mypy");
        map.insert("pylint", "pylint");
        map.insert("ruff", "ruff");

        // JavaScript/TypeScript
        map.insert("prettier", "prettier");
        map.insert("eslint", "eslint");
        map.insert("standard", "standard_js");

        // Rust
        map.insert("rustfmt", "rustfmt");
        map.insert("cargo-fmt", "cargo_fmt");
        map.insert("clippy", "cargo_clippy");
        map.insert("cargo-check", "cargo_check");
        map.insert("fmt", "rustfmt");

        // Go
        map.insert("gofmt", "go_fmt");
        map.insert("goimports", "go_imports");
        map.insert("golangci-lint", "golangci_lint");
        map.insert("go-vet", "go_vet");

        // Ruby
        map.insert("rubocop", "rubocop");

        // Shell
        map.insert("shellcheck", "shellcheck");
        map.insert("shfmt", "shfmt");

        // YAML
        map.insert("yamllint", "yamllint");

        // Docker
        map.insert("hadolint", "hadolint");

        // Terraform
        map.insert("terraform-fmt", "terraform");
        map.insert("tflint", "tf_lint");

        // CSS
        map.insert("stylelint", "stylelint");

        // Markdown
        map.insert("markdownlint", "markdown_lint");

        // GitHub Actions
        map.insert("actionlint", "actionlint");

        // pre-commit-hooks utilities
        map.insert("trailing-whitespace", "trailing_whitespace");
        map.insert("end-of-file-fixer", "newlines");
        map.insert("check-yaml", "yamllint");
        map.insert("check-json", "jq");
        map.insert("check-toml", "taplo");
        map.insert("check-merge-conflict", "check_merge_conflict");
        map.insert("check-case-conflict", "check_case_conflict");
        map.insert("mixed-line-ending", "mixed_line_ending");
        map.insert(
            "check-executables-have-shebangs",
            "check_executables_have_shebangs",
        );
        map.insert("check-symlinks", "check_symlinks");
        map.insert("check-byte-order-marker", "check_byte_order_marker");
        map.insert("check-added-large-files", "check_added_large_files");
        map.insert("check-ast", "python_check_ast");
        map.insert("debug-statements", "python_debug_statements");
        map.insert("detect-private-key", "detect_private_key");
        map.insert("no-commit-to-branch", "no_commit_to_branch");
        map.insert("fix-byte-order-marker", "fix_byte_order_marker");

        map
    }

    /// Vendor external repositories referenced in the config
    async fn vendor_repos(
        &self,
        config: &PreCommitConfig,
    ) -> Result<HashMap<String, VendoredRepo>> {
        let mut vendored = HashMap::new();

        for repo in &config.repos {
            // Skip local and meta repos, and repos that don't need vendoring
            if repo.repo == "local"
                || repo.repo == "meta"
                || Self::is_github_precommit_hooks(&repo.repo)
            {
                continue;
            }

            // Check if we need to vendor this repo (if it has unknown hooks)
            let needs_vendoring = repo.hooks.iter().any(|h| !self.is_known_hook(&h.id));

            if !needs_vendoring {
                continue;
            }

            // Create vendor directory structure
            let vendor_name = Self::repo_url_to_vendor_name(&repo.repo);
            let vendor_path = PathBuf::from(".hk/vendors").join(&vendor_name);

            // Create the vendor directory
            std::fs::create_dir_all(&vendor_path)?;

            info!("Vendoring repository: {}", repo.repo);

            // Clone or download the repo
            if let Err(e) = self
                .download_repo(&repo.repo, repo.rev.as_deref(), &vendor_path)
                .await
            {
                warn!(
                    "Failed to vendor {}: {}. Hooks will need manual configuration.",
                    repo.repo, e
                );
                // Clean up partial clone
                let _ = std::fs::remove_dir_all(&vendor_path);
                continue;
            }

            // Remove .git directory to save space
            let git_dir = vendor_path.join(".git");
            if git_dir.exists() {
                let _ = std::fs::remove_dir_all(&git_dir);
            }

            // Make scripts executable
            Self::make_scripts_executable(&vendor_path)?;

            // Parse the .pre-commit-hooks.yaml file
            let hooks_yaml_path = vendor_path.join(".pre-commit-hooks.yaml");
            let hooks = if hooks_yaml_path.exists() {
                let yaml_content = xx::file::read_to_string(&hooks_yaml_path)?;
                serde_yaml::from_str::<Vec<PreCommitHookDefinition>>(&yaml_content)?
            } else {
                warn!(
                    "No .pre-commit-hooks.yaml found in {}, generating basic wrappers",
                    repo.repo
                );
                // Generate basic hook definitions from the config
                repo.hooks
                    .iter()
                    .map(|h| PreCommitHookDefinition {
                        id: h.id.clone(),
                        name: h.name.clone(),
                        entry: h.entry.clone().unwrap_or_else(|| h.id.clone()),
                        language: h.language.clone().unwrap_or_else(|| "system".to_string()),
                        files: h.files.clone(),
                        types: h.types.clone(),
                        pass_filenames: h.pass_filenames,
                    })
                    .collect()
            };

            // Generate the hooks.pkl file for this vendor
            self.generate_vendor_pkl(&vendor_path, &hooks, &repo.repo)?;

            vendored.insert(
                repo.repo.clone(),
                VendoredRepo {
                    url: repo.repo.clone(),
                    name: vendor_name,
                    vendor_path,
                    hooks,
                },
            );
        }

        Ok(vendored)
    }

    /// Check if this is the standard pre-commit hooks repo
    fn is_github_precommit_hooks(url: &str) -> bool {
        url.contains("github.com/pre-commit/pre-commit-hooks")
            || url.contains("github.com/pre-commit/mirrors-")
            || url.contains("github.com/psf/")
            || url.contains("github.com/PyCQA/")
            || url.contains("github.com/asottile/")
    }

    /// Download a repository to the vendor path
    async fn download_repo(&self, url: &str, rev: Option<&str>, dest: &Path) -> Result<()> {
        // Use git clone for GitHub URLs
        if url.starts_with("https://") || url.starts_with("git@") {
            let mut cmd = std::process::Command::new("git");
            cmd.arg("clone");
            cmd.arg("--depth=1");

            if let Some(rev) = rev {
                cmd.arg("--branch").arg(rev);
            }

            cmd.arg(url).arg(dest);

            let output = cmd.output()?;
            if !output.status.success() {
                bail!(
                    "Failed to clone repository {}: {}",
                    url,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        } else {
            bail!("Unsupported repository URL format: {}", url);
        }

        Ok(())
    }

    /// Convert a repository URL to a vendor directory name
    fn repo_url_to_vendor_name(url: &str) -> String {
        // Extract repo name from URL
        // e.g., https://github.com/Lucas-C/pre-commit-hooks -> Lucas-C-pre-commit-hooks
        url.trim_end_matches('/')
            .split('/')
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("-")
            .replace('.', "-")
    }

    /// Convert a repository URL to an import name
    fn repo_url_to_import_name(url: &str) -> String {
        // Convert to valid Pkl identifier
        // e.g., https://github.com/Lucas-C/pre-commit-hooks -> Vendors_Lucas_C_pre_commit_hooks
        let vendor_name = Self::repo_url_to_vendor_name(url).replace(['-', '.'], "_");
        format!(
            "Vendors_{}",
            vendor_name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>()
        )
    }

    /// Generate a hooks.pkl file for a vendored repository
    fn generate_vendor_pkl(
        &self,
        vendor_path: &Path,
        hooks: &[PreCommitHookDefinition],
        repo_url: &str,
    ) -> Result<()> {
        let version = env!("CARGO_PKG_VERSION");
        let mut pkl_content = String::new();
        pkl_content.push_str("// Auto-generated hooks from vendored repository\n");
        pkl_content.push_str(&format!("// Source: {}\n\n", repo_url));

        // Use package URL for Config.pkl to match the main hk.pkl
        if let Some(ref root) = self.hk_pkl_root {
            // Vendor hooks.pkl is at .hk/vendors/<vendor-name>/hooks.pkl
            // So we need to go up 3 levels (../../..) to reach project root
            // then apply the hk_pkl_root path
            let vendor_path = if root.starts_with("../") {
                // Convert ../pkl to ../../../../pkl (3 more ../ for vendor directory depth)
                // Remove the leading ../ and add ../../../../
                let without_prefix = root.strip_prefix("../").unwrap_or(root);
                format!("../../../../{}", without_prefix)
            } else {
                // If absolute or package URL, use as-is
                root.clone()
            };
            pkl_content.push_str(&format!("import \"{}/Config.pkl\"\n\n", vendor_path));
        } else {
            pkl_content.push_str(&format!(
                "import \"package://github.com/jdx/hk/releases/download/v{}/hk@{}#/Config.pkl\"\n\n",
                version, version
            ));
        }

        for hook in hooks {
            let hook_id_snake = hook.id.replace('-', "_");
            pkl_content.push_str(&format!("{} = new Config.Step {{\n", hook_id_snake));

            // Add glob pattern if specified
            if let Some(ref files) = hook.files {
                let escaped_files = Self::escape_for_pkl(files);
                pkl_content.push_str(&format!("    glob = \"{}\"\n", escaped_files));
            }

            // Build command path
            let pass_filenames = hook.pass_filenames.unwrap_or(true);

            // For pygrep hooks, use grep with the pattern
            let (cmd, needs_prefix) = if hook.language == "pygrep" {
                // The entry is a regex pattern - use rg (ripgrep) with PCRE2 for advanced regex features
                let pattern = &hook.entry;
                let cmd = if pass_filenames {
                    format!("rg --color=never --pcre2 -n '{}' {{{{files}}}}", pattern)
                } else {
                    format!("rg --color=never --pcre2 -n '{}'", pattern)
                };
                (cmd, false) // No prefix needed
            } else if hook.language == "python" {
                // Check if this is a local Python script (starts with ./ or ../)
                let entry_path = vendor_path.join(&hook.entry);
                if entry_path.exists() && hook.entry.ends_with(".py") {
                    // Use uv run for local Python scripts - it handles both regular scripts and PEP 723
                    let relative_entry = format!(
                        ".hk/vendors/{}/{}",
                        Self::repo_url_to_vendor_name(repo_url),
                        hook.entry
                    );
                    let cmd = if pass_filenames {
                        format!("uv run {} {{{{files}}}}", relative_entry)
                    } else {
                        format!("uv run {}", relative_entry)
                    };
                    (cmd, false) // No prefix needed, uv run handles dependencies
                } else {
                    // Try to find the Python module based on the entry point
                    let module_name = Self::find_python_module(vendor_path, &hook.entry);
                    if let Some(module) = module_name {
                        let vendor_name = Self::repo_url_to_vendor_name(repo_url);
                        // Use uv to install dependencies if needed, then run the module
                        let install_check = format!(
                            "[ -d .hk/vendors/{}/.venv ] || (cd .hk/vendors/{} && uv venv && uv pip install -e .)",
                            vendor_name, vendor_name
                        );
                        // Use absolute path to python to avoid cd which breaks relative file paths
                        let python_path = format!(".hk/vendors/{}/.venv/bin/python", vendor_name);
                        let module_path = if pass_filenames {
                            format!(
                                "{} && {} -m {} {{{{files}}}}",
                                install_check, python_path, module
                            )
                        } else {
                            format!("{} && {} -m {}", install_check, python_path, module)
                        };
                        (module_path, false) // No prefix needed, we're calling python directly
                    } else {
                        // Fallback to entry name with prefix
                        let cmd = if pass_filenames {
                            format!("{} {{{{files}}}}", hook.entry)
                        } else {
                            hook.entry.clone()
                        };
                        (cmd, true) // Need mise x prefix
                    }
                }
            } else if hook.language == "node" {
                // For Node.js hooks, try to find the package and use npx
                let vendor_name = Self::repo_url_to_vendor_name(repo_url);
                let package_name = Self::find_node_package_name(vendor_path);

                let node_cmd = if let Some(pkg_name) = package_name {
                    // Check for node_modules and install if needed, then run npx
                    let install_check = format!(
                        "[ -d .hk/vendors/{}/node_modules ] || (cd .hk/vendors/{} && npm install --silent --no-audit --no-fund)",
                        vendor_name, vendor_name
                    );
                    if pass_filenames {
                        format!(
                            "{} && npx --prefix .hk/vendors/{} {} {{{{files}}}}",
                            install_check, vendor_name, pkg_name
                        )
                    } else {
                        format!(
                            "{} && npx --prefix .hk/vendors/{} {}",
                            install_check, vendor_name, pkg_name
                        )
                    }
                } else {
                    // Fallback to entry name
                    if pass_filenames {
                        format!("{} {{{{files}}}}", hook.entry)
                    } else {
                        hook.entry.clone()
                    }
                };
                (node_cmd, false) // No prefix needed, we're calling npx directly
            } else if hook.language == "golang" {
                // For Go hooks, install via go install and use the binary from GOPATH/bin
                let vendor_name = Self::repo_url_to_vendor_name(repo_url);
                // Create isolated GOPATH in the vendor directory
                let gopath = format!(".hk/vendors/{}/.gopath", vendor_name);
                let install_check = format!(
                    "[ -d {}/bin ] || (export GOPATH=$(pwd)/{} && cd .hk/vendors/{} && go install ./...)",
                    gopath, gopath, vendor_name
                );
                // Use the binary name from entry, which should be in GOPATH/bin
                let binary_name = hook.entry.split('/').next_back().unwrap_or(&hook.entry);
                let go_cmd = if pass_filenames {
                    format!(
                        "{} && {}/bin/{} {{{{files}}}}",
                        install_check, gopath, binary_name
                    )
                } else {
                    format!("{} && {}/bin/{}", install_check, gopath, binary_name)
                };
                (go_cmd, false) // No prefix needed, we're calling the binary directly
            } else if hook.language == "ruby" {
                // For Ruby hooks, build and install the gem
                let vendor_name = Self::repo_url_to_vendor_name(repo_url);
                let gem_home = format!(".hk/vendors/{}/.gem-home", vendor_name);
                let install_check = format!(
                    "[ -d {}/bin ] || (cd .hk/vendors/{} && gem build *.gemspec && gem install --no-document --install-dir $(pwd)/.gem-home --bindir $(pwd)/.gem-home/bin *.gem)",
                    gem_home, vendor_name
                );
                // Use the entry as the binary name, with GEM_HOME set and bin directory in PATH
                let ruby_cmd = if pass_filenames {
                    format!(
                        "{} && GEM_HOME=$(pwd)/{} GEM_PATH= PATH=$(pwd)/{}/bin:$PATH {}/bin/{} {{{{files}}}}",
                        install_check, gem_home, gem_home, gem_home, hook.entry
                    )
                } else {
                    format!(
                        "{} && GEM_HOME=$(pwd)/{} GEM_PATH= PATH=$(pwd)/{}/bin:$PATH {}/bin/{}",
                        install_check, gem_home, gem_home, gem_home, hook.entry
                    )
                };
                (ruby_cmd, false) // No prefix needed, we're calling the binary directly
            } else if hook.language == "swift" {
                // For Swift hooks, build the package and use the binary from .build/release
                let vendor_name = Self::repo_url_to_vendor_name(repo_url);
                let build_dir = format!(".hk/vendors/{}/.swift_env/.build/release", vendor_name);
                let install_check = format!(
                    "[ -d {} ] || (cd .hk/vendors/{} && swift build -c release --build-path .swift_env/.build)",
                    build_dir, vendor_name
                );
                // Extract the binary name from entry (e.g., "swift-format format --in-place" -> "swift-format")
                let binary_name = hook.entry.split_whitespace().next().unwrap_or(&hook.entry);
                // Get any additional arguments from entry
                let entry_args = hook
                    .entry
                    .split_whitespace()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join(" ");
                let swift_cmd = if pass_filenames {
                    if entry_args.is_empty() {
                        format!(
                            "{} && {}/{} {{{{files}}}}",
                            install_check, build_dir, binary_name
                        )
                    } else {
                        format!(
                            "{} && {}/{} {} {{{{files}}}}",
                            install_check, build_dir, binary_name, entry_args
                        )
                    }
                } else if entry_args.is_empty() {
                    format!("{} && {}/{}", install_check, build_dir, binary_name)
                } else {
                    format!(
                        "{} && {}/{} {}",
                        install_check, build_dir, binary_name, entry_args
                    )
                };
                (swift_cmd, false) // No prefix needed, we're calling the binary directly
            } else {
                // For non-Python/Node/Go/Ruby/Swift hooks, use the entry directly
                let entry_path = vendor_path.join(&hook.entry);
                let relative_entry = if entry_path.exists() {
                    format!(
                        ".hk/vendors/{}/{}",
                        Self::repo_url_to_vendor_name(repo_url),
                        hook.entry
                    )
                } else {
                    hook.entry.clone()
                };

                let cmd = if pass_filenames {
                    format!("{} {{{{files}}}}", relative_entry)
                } else {
                    relative_entry.clone()
                };

                // Determine if prefix is needed based on language
                let needs_prefix = matches!(hook.language.as_str(), "node" | "ruby" | "rust");
                (cmd, needs_prefix)
            };

            // Add prefix if needed
            if needs_prefix {
                match hook.language.as_str() {
                    "python" => {
                        pkl_content.push_str("    prefix = \"mise x python@latest --\"\n");
                    }
                    "node" => {
                        pkl_content.push_str("    prefix = \"mise x node@latest --\"\n");
                    }
                    _ => {}
                }
            }

            // Detect if this is a fixer or checker based on hook name/description
            let is_fixer = Self::is_fixer_hook(&hook.id, hook.name.as_deref());

            // Escape the command for Pkl string literals
            let escaped_cmd = Self::escape_for_pkl(&cmd);

            if is_fixer {
                // Fixers get both check and fix commands
                pkl_content.push_str(&format!("    check = \"{}\"\n", escaped_cmd));
                pkl_content.push_str(&format!("    fix = \"{}\"\n", escaped_cmd));
            } else {
                // Checkers only get check command
                pkl_content.push_str(&format!("    check = \"{}\"\n", escaped_cmd));
            }

            pkl_content.push_str("}\n\n");
        }

        let pkl_path = vendor_path.join("hooks.pkl");
        xx::file::write(pkl_path, pkl_content)?;

        Ok(())
    }

    /// Escape a string for use in Pkl string literals
    /// Pkl supports these escape sequences: \n \r \t \" \\
    /// We need to escape backslashes so that regex patterns like \[ become \\[
    /// Also escape newlines as \n since Pkl strings must be on a single line
    fn escape_for_pkl(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Detect if a hook is a fixer (modifies files) or a checker (read-only)
    /// based on naming conventions
    fn is_fixer_hook(id: &str, name: Option<&str>) -> bool {
        // Common patterns indicating a fixer
        let fixer_patterns = [
            "remove-",
            "fix-",
            "format-",
            "sort-",
            "insert-",
            "add-",
            "delete-",
            "replace-",
            "reorder-",
            "normalize-",
            "trim-",
            "strip-",
            "clean-",
            "update-",
            "transform-",
        ];

        let checker_patterns = [
            "forbid-",
            "check-",
            "detect-",
            "validate-",
            "verify-",
            "lint-",
            "scan-",
            "find-",
        ];

        // Check ID for patterns
        for pattern in &fixer_patterns {
            if id.starts_with(pattern) {
                return true;
            }
        }

        for pattern in &checker_patterns {
            if id.starts_with(pattern) {
                return false;
            }
        }

        // Check name if available
        if let Some(name_str) = name {
            let name_lower = name_str.to_lowercase();
            if name_lower.contains("remover")
                || name_lower.contains("fixer")
                || name_lower.contains("formatter")
                || name_lower.contains("replace")
                || name_lower.contains("format ")
            {
                return true;
            }

            if name_lower.contains("checker")
                || name_lower.contains("validator")
                || name_lower.contains("linter")
            {
                return false;
            }
        }

        // Default to checker (safer - doesn't modify files)
        false
    }

    /// Make Python scripts and other executable files in the vendor directory executable
    fn make_scripts_executable(vendor_path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        // Find all Python files and shell scripts
        if let Ok(entries) = std::fs::read_dir(vendor_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "py" || ext == "sh" {
                            // Make executable
                            if let Ok(metadata) = std::fs::metadata(&path) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(perms.mode() | 0o111); // Add execute permission
                                let _ = std::fs::set_permissions(&path, perms);
                            }
                        }
                    }
                }
            }
        }

        // Also check pre_commit_hooks subdirectory if it exists
        let hooks_dir = vendor_path.join("pre_commit_hooks");
        if hooks_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&hooks_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            if ext == "py" {
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    let mut perms = metadata.permissions();
                                    perms.set_mode(perms.mode() | 0o111);
                                    let _ = std::fs::set_permissions(&path, perms);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Find the Python module name for a given entry point
    /// E.g., "forbid_crlf" -> Some("pre_commit_hooks.forbid_crlf")
    fn find_python_module(vendor_path: &Path, entry: &str) -> Option<String> {
        // Check if there's a pre_commit_hooks directory with the module
        let hooks_dir = vendor_path.join("pre_commit_hooks");
        if hooks_dir.exists() {
            // Convert entry to potential module name
            let module_file = format!("{}.py", entry);
            if hooks_dir.join(&module_file).exists() {
                return Some(format!("pre_commit_hooks.{}", entry));
            }
        }

        // Check for other common Python package structures
        // Look for setup.py to parse entry_points (basic parsing)
        let setup_py = vendor_path.join("setup.py");
        if setup_py.exists() {
            if let Ok(content) = std::fs::read_to_string(&setup_py) {
                // Look for entry_points console_scripts
                if let Some(start) = content.find("\"console_scripts\"") {
                    let after_start = &content[start..];
                    // Look for the entry pattern
                    let entry_pattern = format!("{} = ", entry);
                    if let Some(entry_pos) = after_start.find(&entry_pattern) {
                        let after_entry = &after_start[entry_pos + entry_pattern.len()..];
                        // Extract module:function
                        if let Some(end_quote) = after_entry.find('"') {
                            let module_func = &after_entry[..end_quote];
                            // Split on : to get module name
                            if let Some(module) = module_func.split(':').next() {
                                return Some(module.to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Find the Node.js package name from package.json
    fn find_node_package_name(vendor_path: &Path) -> Option<String> {
        let package_json = vendor_path.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                // Parse package.json to find the name
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name") {
                        if let Some(name_str) = name.as_str() {
                            return Some(name_str.to_string());
                        }
                    }
                }
            }
        }
        None
    }
}
