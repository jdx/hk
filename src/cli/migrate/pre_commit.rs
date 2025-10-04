use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::Result;
use eyre::bail;
use indexmap::IndexMap;
use serde::Deserialize;

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
    #[serde(default)]
    exclude: Option<String>,
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

        let (config_pkl, builtins_pkl) = if let Some(ref root) = self.hk_pkl_root {
            (
                Some(format!("{}/Config.pkl", root)),
                Some(format!("{}/Builtins.pkl", root)),
            )
        } else {
            (None, None)
        };

        let mut hk_config = self.convert_config(&precommit_config, config_pkl, builtins_pkl)?;

        // Map pre-commit top-level exclude to hk global excludes
        if let Some(ref exclude) = precommit_config.exclude {
            // If the pattern looks like a verbose regex (?x), collapse spaces/comments (handled upstream already),
            // but for now just carry the pattern verbatim and let PKL config use regex strings where needed.
            // Since hk excludes are glob-based, when regex cannot be converted we still include as-is for now.
            hk_config.global_excludes.push(exclude.clone());
        }
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
        println!("2. Complete any TODO items for local/unknown hooks");
        println!("3. Run 'hk install' to install git hooks");
        println!("4. Run 'hk check --all' to test your configuration");

        Ok(())
    }

    fn convert_config(
        &self,
        config: &PreCommitConfig,
        config_pkl: Option<String>,
        builtins_pkl: Option<String>,
    ) -> Result<HkConfig> {
        let mut hk_config = HkConfig::new(config_pkl, builtins_pkl);
        let mut used_ids = HashSet::new();

        // Add header comments
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
            for (lang, version) in &config.default_language_version {
                hk_config
                    .header_comments
                    .push(format!("  {}: {}", lang, version));
            }
            hk_config
                .header_comments
                .push("Note: Use mise.toml to manage language versions in hk".to_string());
        }

        // Initialize step collections
        let mut linters = IndexMap::new();
        let mut local_hooks = IndexMap::new();
        let mut custom_steps = IndexMap::new();

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

                // Convert the hook to an HkStep
                let step = if is_local {
                    self.convert_local_hook(hook, &unique_id, repo)
                } else if let Some(step) = self.convert_known_hook(hook, &unique_id) {
                    step
                } else {
                    self.convert_unknown_hook(hook, &unique_id, repo)
                };

                // Add to appropriate collection
                let collection_name = if is_local {
                    local_hooks.insert(unique_id.clone(), step);
                    "local_hooks"
                } else if self.is_known_hook(&hook.id) {
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

        // Generate hooks
        let has_steps = !hk_config.step_collections.is_empty();

        // Create hooks for each stage
        let mut stages_used = HashSet::new();
        for (stage, _) in &steps_by_stage {
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

            for collection in &["linters", "local_hooks", "custom_steps"] {
                if collections_in_stage.contains(*collection) {
                    hook.step_spreads.push(collection.to_string());
                }
            }

            hk_config.hooks.insert(stage.clone(), hook);
        }

        // Always add check and fix hooks if we have any steps
        if has_steps {
            // Check hook
            let mut check_hook = HkHook {
                fix: None,
                stash: None,
                step_spreads: Vec::new(),
                direct_steps: IndexMap::new(),
            };

            for collection in &["linters", "local_hooks", "custom_steps"] {
                if hk_config.step_collections.contains_key(*collection) {
                    check_hook.step_spreads.push(collection.to_string());
                }
            }

            hk_config.hooks.insert("check".to_string(), check_hook);

            // Fix hook
            let mut fix_hook = HkHook {
                fix: Some(true),
                stash: None,
                step_spreads: Vec::new(),
                direct_steps: IndexMap::new(),
            };

            for collection in &["linters", "local_hooks"] {
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
                exclude: hook.exclude.clone(),
                prefix: None,
                check: None,
                fix: None,
                shell: None,
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
            exclude: hook.exclude.clone(),
            prefix: None,
            check: None,
            fix: None,
            shell: None,
            properties_as_comments: Vec::new(),
        };

        // Add comments
        if unique_id != hook.id {
            step.comments.push(format!("Original ID: {}", hook.id));
        }
        if let Some(ref name) = hook.name {
            step.comments.push(format!("Name: {}", name));
        }

        // Handle additional_dependencies with mise x
        if !hook.additional_dependencies.is_empty() {
            step.prefix = Self::generate_mise_prefix(&hook.additional_dependencies);
        }

        // Set check command
        if let Some(ref entry) = hook.entry {
            let pass_filenames = hook.pass_filenames.unwrap_or(true);
            if pass_filenames {
                step.check = Some(format!("{} {{{{files}}}}", entry));
            } else {
                step.check = Some(entry.clone());
            }

            if !hook.args.is_empty() {
                step.properties_as_comments
                    .push(format!("args from pre-commit: {}", hook.args.join(" ")));
                step.properties_as_comments
                    .push("Consider adding args to check command".to_string());
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
    ) -> HkStep {
        let mut step = HkStep {
            builtin: None,
            comments: Vec::new(),
            glob: None,
            exclude: hook.exclude.clone(),
            prefix: None,
            check: None,
            fix: None,
            shell: None,
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
            _ => "pre-commit",
        }
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

    fn hook_id_to_tool(&self, hook_id: &str) -> String {
        match hook_id {
            "black" | "flake8" | "isort" | "mypy" | "pylint" => hook_id.to_string(),
            "ruff" | "ruff-check" | "ruff-format" => "ruff".to_string(),
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
        map.insert("ruff-check", "ruff");
        map.insert("ruff-format", "ruff");

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
}
