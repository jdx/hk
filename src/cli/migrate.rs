use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::Result;
use eyre::bail;
use indexmap::IndexMap;
use serde::Deserialize;

/// Migrate from other hook managers to hk
#[derive(Debug, clap::Args)]
pub struct Migrate {
    #[clap(subcommand)]
    command: MigrateCommands,
}

#[derive(Debug, clap::Subcommand)]
enum MigrateCommands {
    /// Migrate from pre-commit to hk
    Precommit(Precommit),
}

/// Migrate from pre-commit to hk
#[derive(Debug, clap::Args)]
pub struct Precommit {
    /// Path to .pre-commit-config.yaml
    #[clap(short, long, default_value = ".pre-commit-config.yaml")]
    config: PathBuf,
    /// Output path for hk.pkl
    #[clap(short, long, default_value = "hk.pkl")]
    output: PathBuf,
    /// Overwrite existing hk.pkl file
    #[clap(short, long)]
    force: bool,
}

impl Migrate {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            MigrateCommands::Precommit(cmd) => cmd.run().await,
        }
    }
}

#[derive(Debug, Deserialize)]
struct PrecommitConfig {
    repos: Vec<PrecommitRepo>,
    #[serde(default)]
    fail_fast: bool,
    #[serde(default)]
    default_language_version: HashMap<String, String>,
    #[serde(default)]
    default_stages: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PrecommitRepo {
    repo: String,
    #[serde(default)]
    rev: Option<String>,
    hooks: Vec<PrecommitHook>,
}

#[derive(Debug, Deserialize)]
struct PrecommitHook {
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

impl Precommit {
    /// Format a string value for Pkl, using custom delimiters if needed
    fn format_pkl_string(value: &str) -> String {
        if value.contains('\n') {
            // Multi-line string, use triple quotes
            format!("#\"\"\"\n{}\n\"\"\"#", value)
        } else if value.contains('\\') || value.contains('"') {
            // String with backslashes or quotes, use custom delimiters
            format!("#\"{}\"#", value)
        } else {
            // Simple string, use regular quotes
            format!("\"{}\"", value)
        }
    }

    /// Format a string value with {{files}} placeholder for Pkl
    fn format_pkl_string_with_files(value: &str) -> String {
        if value.contains('\n') {
            // Multi-line string, use triple quotes
            format!("#\"\"\"\n{} {{{{files}}}}\n\"\"\"#", value)
        } else if value.contains('\\') || value.contains('"') {
            // String with backslashes or quotes, use custom delimiters
            format!("#\"{} {{{{files}}}}\"#", value)
        } else {
            // Simple string, use regular quotes
            format!("\"{} {{{{files}}}}\"", value)
        }
    }

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
        let precommit_config: PrecommitConfig = serde_yaml::from_str(&config_content)?;

        let (hk_config, _tools) = self.convert_config(&precommit_config)?;
        xx::file::write(&self.output, hk_config)?;

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

    fn convert_config(&self, config: &PrecommitConfig) -> Result<(String, HashSet<String>)> {
        let mut output = String::new();
        let mut tools = HashSet::new();

        output.push_str(
            r#"amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Builtins.pkl"

"#,
        );

        if config.fail_fast {
            output.push_str("// Migrated from pre-commit fail_fast setting\n");
            output.push_str("// Note: hk uses --fail-fast CLI flag instead of config setting\n\n");
        }

        if !config.default_language_version.is_empty() {
            output.push_str("// pre-commit default_language_version:\n");
            for (lang, version) in &config.default_language_version {
                output.push_str(&format!("//   {}: {}\n", lang, version));
            }
            output.push_str("// Note: Use mise.toml to manage language versions in hk\n\n");
        }

        let mut steps_by_stage: IndexMap<String, Vec<String>> = IndexMap::new();
        let mut unknown_hooks = Vec::new();
        let mut local_hooks = Vec::new();
        let mut used_ids = HashSet::new();

        for repo in &config.repos {
            let is_local = repo.repo == "local";
            let is_meta = repo.repo == "meta";

            for hook in &repo.hooks {
                let unique_id = Self::make_unique_hook_id(&hook.id, &mut used_ids);

                let hook_stages = if !hook.stages.is_empty() {
                    hook.stages.clone()
                } else if !config.default_stages.is_empty() {
                    config.default_stages.clone()
                } else {
                    vec!["pre-commit".to_string()]
                };

                if is_local {
                    local_hooks.push((hook, repo, hook_stages.clone(), unique_id.clone()));
                    continue;
                }

                if is_meta {
                    // Skip meta hooks, they're pre-commit internal
                    continue;
                }

                let conversion_result =
                    self.convert_hook(hook, &unique_id, repo, config, &mut tools);

                for stage in hook_stages {
                    let stage_clone = stage.clone();
                    let steps = steps_by_stage.entry(stage).or_default();

                    if let Some(ref step) = conversion_result {
                        steps.push(step.clone());
                    } else {
                        // Track unknown hooks separately
                        if !unknown_hooks.iter().any(
                            |(h, _, _, _): &(&PrecommitHook, &PrecommitRepo, _, String)| {
                                h.id == hook.id
                            },
                        ) {
                            unknown_hooks.push((hook, repo, stage_clone, unique_id.clone()));
                        }
                    }
                }
            }
        }

        // Generate linters mapping
        let all_known_steps: Vec<String> = steps_by_stage
            .values()
            .flatten()
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        if !all_known_steps.is_empty() {
            output.push_str("local linters = new Mapping<String, Step> {\n");
            for step in &all_known_steps {
                output.push_str(step);
            }
            output.push_str("}\n\n");
        }

        // Generate local hooks
        if !local_hooks.is_empty() {
            output.push_str("// Local hooks from your pre-commit config\n");
            output.push_str("local local_hooks = new Mapping<String, Step> {\n");
            for (hook, repo, _, unique_id) in &local_hooks {
                output.push_str(&self.generate_local_hook(hook, unique_id, repo));
            }
            output.push_str("}\n\n");
        }

        // Generate unknown hooks
        if !unknown_hooks.is_empty() {
            output.push_str("// The following hooks could not be automatically converted.\n");
            output.push_str("// Please configure them manually or use equivalent hk builtins.\n");
            output.push_str("local custom_steps = new Mapping<String, Step> {\n");
            for (hook, repo, _, unique_id) in &unknown_hooks {
                output.push_str(&self.generate_custom_step(hook, unique_id, repo));
            }
            output.push_str("}\n\n");
        }

        // Generate hooks configuration
        output.push_str("hooks {\n");

        // Group by stage
        for (stage, _steps) in steps_by_stage {
            let hk_stage = match stage.as_str() {
                "commit" | "commit-msg" => "commit-msg",
                "push" | "pre-push" => "pre-push",
                "prepare-commit-msg" => "prepare-commit-msg",
                _ => "pre-commit",
            };

            output.push_str(&format!("    [\"{}\"] {{\n", hk_stage));

            if hk_stage == "pre-commit" {
                output.push_str("        fix = true\n");
                output.push_str("        stash = \"git\"\n");
            }

            output.push_str("        steps {\n");

            if !all_known_steps.is_empty() {
                output.push_str("            ...linters\n");
            }

            if !local_hooks.is_empty()
                && local_hooks
                    .iter()
                    .any(|(_, _, stages, _)| stages.contains(&stage))
            {
                output.push_str("            ...local_hooks\n");
            }

            if !unknown_hooks.is_empty() && unknown_hooks.iter().any(|(_, _, s, _)| s == &stage) {
                output.push_str("            ...custom_steps\n");
            }

            output.push_str("        }\n");
            output.push_str("    }\n");
        }

        // Always add check and fix hooks
        if !all_known_steps.is_empty() || !local_hooks.is_empty() {
            output.push_str("    [\"check\"] {\n");
            output.push_str("        steps {\n");
            if !all_known_steps.is_empty() {
                output.push_str("            ...linters\n");
            }
            if !local_hooks.is_empty() {
                output.push_str("            ...local_hooks\n");
            }
            if !unknown_hooks.is_empty() {
                output.push_str("            ...custom_steps\n");
            }
            output.push_str("        }\n");
            output.push_str("    }\n");

            output.push_str("    [\"fix\"] {\n");
            output.push_str("        fix = true\n");
            output.push_str("        steps {\n");
            if !all_known_steps.is_empty() {
                output.push_str("            ...linters\n");
            }
            if !local_hooks.is_empty() {
                output.push_str("            ...local_hooks\n");
            }
            output.push_str("        }\n");
            output.push_str("    }\n");
        }

        output.push_str("}\n");

        Ok((output, tools))
    }

    fn convert_hook(
        &self,
        hook: &PrecommitHook,
        unique_id: &str,
        _repo: &PrecommitRepo,
        _config: &PrecommitConfig,
        tools: &mut HashSet<String>,
    ) -> Option<String> {
        let builtin_map = self.get_builtin_map();

        if let Some(builtin_name) = builtin_map.get(hook.id.as_str()) {
            tools.insert(self.hook_id_to_tool(&hook.id));

            let mut step = format!("    [\"{}\"] = ", unique_id);

            let needs_customization = hook.files.is_some()
                || hook.exclude.is_some()
                || !hook.args.is_empty()
                || !hook.additional_dependencies.is_empty()
                || !hook.types.is_empty()
                || !hook.types_or.is_empty()
                || !hook.exclude_types.is_empty()
                || hook.always_run
                || hook.pass_filenames == Some(false)
                || hook.language_version.is_some();

            if needs_customization {
                step.push_str(&format!("(Builtins.{}) {{\n", builtin_name));

                if let Some(ref files) = hook.files {
                    step.push_str(&format!(
                        "        // files pattern from pre-commit: {}\n",
                        files
                    ));
                    step.push_str("        // Note: Convert regex to glob pattern for hk\n");
                }

                if let Some(ref exclude) = hook.exclude {
                    step.push_str(&format!(
                        "        exclude = {}\n",
                        Self::format_pkl_string(exclude)
                    ));
                }

                if !hook.types.is_empty() {
                    step.push_str(&format!(
                        "        // types (AND): {}\n",
                        hook.types.join(", ")
                    ));
                }

                if !hook.types_or.is_empty() {
                    step.push_str(&format!(
                        "        // types_or: {}\n",
                        hook.types_or.join(", ")
                    ));
                }

                if !hook.exclude_types.is_empty() {
                    step.push_str(&format!(
                        "        // exclude_types: {}\n",
                        hook.exclude_types.join(", ")
                    ));
                }

                if hook.always_run {
                    step.push_str(
                        "        // always_run: true - runs even without matching files\n",
                    );
                    step.push_str("        // Note: hk doesn't have direct equivalent, hook will run on all files\n");
                }

                if hook.pass_filenames == Some(false) {
                    step.push_str("        // pass_filenames: false\n");
                    step.push_str(
                        "        // Note: Adjust check/fix commands to not use {{files}}\n",
                    );
                }

                if !hook.args.is_empty() {
                    let args_str = hook.args.join(" ");
                    step.push_str(&format!("        // args from pre-commit: {}\n", args_str));
                    step.push_str(
                        "        // Consider updating check/fix commands with these args\n",
                    );
                }

                if !hook.additional_dependencies.is_empty() {
                    let deps = hook.additional_dependencies.join(", ");
                    step.push_str(&format!("        // additional_dependencies: {}\n", deps));

                    // Generate mise x wrapper
                    let tool_name = self.hook_id_to_tool(&hook.id);
                    step.push_str("        // Use mise x to install dependencies:\n");
                    step.push_str(&format!(
                        "        prefix = \"mise x {}@latest --\"\n",
                        tool_name
                    ));
                }

                if let Some(ref lang_ver) = hook.language_version {
                    step.push_str(&format!("        // language_version: {}\n", lang_ver));
                    step.push_str("        // Configure version in mise.toml\n");
                }

                step.push_str("    }\n");
            } else {
                step.push_str(&format!("Builtins.{}\n", builtin_name));
            }

            Some(step)
        } else {
            None
        }
    }

    fn generate_local_hook(
        &self,
        hook: &PrecommitHook,
        unique_id: &str,
        _repo: &PrecommitRepo,
    ) -> String {
        // Add comment if ID was changed
        let mut step = String::new();
        if unique_id != hook.id {
            step.push_str(&format!("    // Original ID: {}\n", hook.id));
        }

        step.push_str(&format!("    [\"{}\"] {{\n", unique_id));

        if let Some(ref name) = hook.name {
            step.push_str(&format!("        // Name: {}\n", name));
        }

        if let Some(ref files) = hook.files {
            step.push_str(&format!(
                "        glob = {}\n",
                Self::format_pkl_string(files)
            ));
        }

        if let Some(ref exclude) = hook.exclude {
            step.push_str(&format!(
                "        exclude = {}\n",
                Self::format_pkl_string(exclude)
            ));
        }

        if hook.always_run {
            step.push_str("        // always_run was true in pre-commit\n");
        }

        // Generate check command from entry
        if let Some(ref entry) = hook.entry {
            let pass_filenames = hook.pass_filenames.unwrap_or(true);

            if pass_filenames {
                step.push_str(&format!(
                    "        check = {}\n",
                    Self::format_pkl_string_with_files(entry)
                ));
            } else {
                step.push_str(&format!(
                    "        check = {}\n",
                    Self::format_pkl_string(entry)
                ));
                step.push_str("        // pass_filenames was false in pre-commit\n");
            }

            if !hook.args.is_empty() {
                step.push_str(&format!(
                    "        // args from pre-commit: {}\n",
                    hook.args.join(" ")
                ));
                step.push_str("        // Consider adding args to check command\n");
            }
        } else {
            step.push_str("        // TODO: Configure check and/or fix commands from local hook\n");
            step.push_str("        // check = \"...\"\n");

            if !hook.args.is_empty() {
                step.push_str(&format!(
                    "        // Original args: {}\n",
                    hook.args.join(" ")
                ));
            }
        }

        step.push_str("    }\n");
        step
    }

    fn generate_custom_step(
        &self,
        hook: &PrecommitHook,
        unique_id: &str,
        repo: &PrecommitRepo,
    ) -> String {
        let mut step = format!("    // Repo: {}", repo.repo);
        if let Some(ref rev) = repo.rev {
            step.push_str(&format!(" @ {}", rev));
        }
        step.push('\n');

        // Add comment if ID was changed
        if unique_id != hook.id {
            step.push_str(&format!("    // Original ID: {}\n", hook.id));
        }

        step.push_str(&format!("    [\"{}\"] {{\n", unique_id));

        if let Some(ref name) = hook.name {
            step.push_str(&format!("        // Name: {}\n", name));
        }

        if let Some(ref files) = hook.files {
            step.push_str(&format!("        // files: {}\n", files));
        }

        if let Some(ref exclude) = hook.exclude {
            step.push_str(&format!(
                "        exclude = {}\n",
                Self::format_pkl_string(exclude)
            ));
        }

        if !hook.types.is_empty() || !hook.types_or.is_empty() {
            step.push_str("        // File type filtering needed\n");
        }

        step.push_str("        // TODO: Configure check and/or fix commands\n");
        step.push_str("        // check = \"...\"\n");
        step.push_str("        // fix = \"...\"\n");

        if !hook.args.is_empty() {
            step.push_str(&format!(
                "        // Original args: {}\n",
                hook.args.join(" ")
            ));
        }

        if !hook.additional_dependencies.is_empty() {
            step.push_str(&format!(
                "        // Dependencies: {}\n",
                hook.additional_dependencies.join(", ")
            ));
        }

        step.push_str("    }\n");
        step
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
        map.insert("ruff-check", "ruff"); // Astral ruff linter
        map.insert("ruff-format", "ruff"); // Astral ruff formatter

        // JavaScript/TypeScript
        map.insert("prettier", "prettier");
        map.insert("eslint", "eslint");
        map.insert("standard", "standard_js");

        // Rust
        map.insert("rustfmt", "rustfmt");
        map.insert("cargo-fmt", "cargo_fmt");
        map.insert("clippy", "cargo_clippy");
        map.insert("cargo-check", "cargo_check");
        map.insert("fmt", "rustfmt"); // doublify/pre-commit-rust

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
