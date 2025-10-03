use std::collections::HashMap;
use std::path::PathBuf;

use crate::Result;
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
    FromPrecommit(FromPrecommit),
}

/// Migrate from pre-commit to hk
#[derive(Debug, clap::Args)]
pub struct FromPrecommit {
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
            MigrateCommands::FromPrecommit(cmd) => cmd.run().await,
        }
    }
}

#[derive(Debug, Deserialize)]
struct PrecommitConfig {
    repos: Vec<PrecommitRepo>,
    #[serde(default)]
    files: Option<String>,
    #[serde(default)]
    exclude: Option<String>,
    #[serde(default)]
    fail_fast: bool,
}

#[derive(Debug, Deserialize)]
struct PrecommitRepo {
    repo: String,
    rev: String,
    hooks: Vec<PrecommitHook>,
}

#[derive(Debug, Deserialize)]
struct PrecommitHook {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    files: Option<String>,
    #[serde(default)]
    exclude: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    additional_dependencies: Vec<String>,
}

impl FromPrecommit {
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

        let hk_config = self.convert_config(&precommit_config)?;
        xx::file::write(&self.output, hk_config)?;

        info!("Migrated {} to {}", self.config.display(), self.output.display());
        println!("Successfully migrated to hk.pkl!");
        println!("\nNext steps:");
        println!("1. Review the generated hk.pkl file");
        println!("2. Install tools referenced in the config (e.g., via mise)");
        println!("3. Run 'hk install' to install git hooks");
        println!("4. Run 'hk check' to test your configuration");

        Ok(())
    }

    fn convert_config(&self, config: &PrecommitConfig) -> Result<String> {
        let mut output = String::new();

        output.push_str(r#"amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Builtins.pkl"

"#);

        if config.fail_fast {
            output.push_str("// Migrated from pre-commit fail_fast setting\n");
            output.push_str("// Note: hk uses --fail-fast flag instead of config setting\n\n");
        }

        let mut steps = Vec::new();
        let mut unknown_hooks = Vec::new();

        for repo in &config.repos {
            for hook in &repo.hooks {
                if let Some(step) = self.convert_hook(hook, repo, config) {
                    steps.push(step);
                } else {
                    unknown_hooks.push((hook, repo));
                }
            }
        }

        if !steps.is_empty() {
            output.push_str("local linters = new Mapping<String, Step> {\n");
            for step in steps {
                output.push_str(&step);
            }
            output.push_str("}\n\n");
        }

        if !unknown_hooks.is_empty() {
            output.push_str("// The following hooks could not be automatically converted.\n");
            output.push_str("// Please configure them manually or use equivalent hk builtins.\n");
            output.push_str("local custom_steps = new Mapping<String, Step> {\n");
            for (hook, repo) in unknown_hooks {
                output.push_str(&self.generate_custom_step(hook, repo));
            }
            output.push_str("}\n\n");
        }

        output.push_str("hooks {\n");
        output.push_str("    [\"pre-commit\"] {\n");
        output.push_str("        fix = true\n");
        output.push_str("        stash = \"git\"\n");
        output.push_str("        steps {\n");

        if !steps.is_empty() {
            output.push_str("            ...linters\n");
        }

        if !unknown_hooks.is_empty() {
            output.push_str("            ...custom_steps\n");
        }

        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push_str("    [\"check\"] {\n");
        output.push_str("        steps {\n");

        if !steps.is_empty() {
            output.push_str("            ...linters\n");
        }

        if !unknown_hooks.is_empty() {
            output.push_str("            ...custom_steps\n");
        }

        output.push_str("        }\n");
        output.push_str("    }\n");
        output.push_str("}\n");

        Ok(output)
    }

    fn convert_hook(
        &self,
        hook: &PrecommitHook,
        repo: &PrecommitRepo,
        _config: &PrecommitConfig,
    ) -> Option<String> {
        // Map of pre-commit hook IDs to hk builtin names
        let builtin_map = self.get_builtin_map();

        if let Some(builtin_name) = builtin_map.get(hook.id.as_str()) {
            let mut step = format!("    [\"{}\"] = ", hook.id);

            // Check if we need customization
            let needs_customization = hook.files.is_some()
                || hook.exclude.is_some()
                || !hook.args.is_empty()
                || !hook.additional_dependencies.is_empty();

            if needs_customization {
                step.push_str(&format!("(Builtins.{}) {{\n", builtin_name));

                if let Some(ref files) = hook.files {
                    step.push_str(&format!("        // files pattern from pre-commit: {}\n", files));
                    step.push_str("        // Note: hk uses glob patterns, you may need to adjust this\n");
                }

                if let Some(ref exclude) = hook.exclude {
                    step.push_str(&format!("        exclude = \"{}\"\n", exclude));
                }

                if !hook.args.is_empty() {
                    step.push_str(&format!("        // args from pre-commit: {}\n", hook.args.join(" ")));
                    step.push_str("        // Note: You may need to adjust the check/fix commands to include these args\n");
                }

                if !hook.additional_dependencies.is_empty() {
                    step.push_str(&format!("        // additional_dependencies: {}\n", hook.additional_dependencies.join(", ")));
                    step.push_str("        // Note: Install these via mise or your package manager\n");
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

    fn generate_custom_step(&self, hook: &PrecommitHook, repo: &PrecommitRepo) -> String {
        let mut step = format!("    // Repo: {} @ {}\n", repo.repo, repo.rev);
        step.push_str(&format!("    [\"{}\"] {{\n", hook.id));

        if let Some(ref name) = hook.name {
            step.push_str(&format!("        // Name: {}\n", name));
        }

        if let Some(ref files) = hook.files {
            step.push_str(&format!("        // files: {}\n", files));
        }

        if let Some(ref exclude) = hook.exclude {
            step.push_str(&format!("        exclude = \"{}\"\n", exclude));
        }

        step.push_str("        // TODO: Configure check and/or fix commands\n");
        step.push_str("        // check = \"...\"\n");
        step.push_str("        // fix = \"...\"\n");

        if !hook.args.is_empty() {
            step.push_str(&format!("        // Original args: {}\n", hook.args.join(" ")));
        }

        if !hook.additional_dependencies.is_empty() {
            step.push_str(&format!("        // Dependencies: {}\n", hook.additional_dependencies.join(", ")));
        }

        step.push_str("    }\n");
        step
    }

    fn get_builtin_map(&self) -> HashMap<&'static str, &'static str> {
        let mut map = HashMap::new();

        // Common pre-commit hooks to hk builtins mapping
        // Python
        map.insert("black", "black");
        map.insert("flake8", "flake8");
        map.insert("isort", "isort");
        map.insert("mypy", "mypy");
        map.insert("pylint", "pylint");
        map.insert("ruff", "ruff");
        map.insert("ruff-format", "ruff");

        // JavaScript/TypeScript
        map.insert("prettier", "prettier");
        map.insert("eslint", "eslint");
        map.insert("standard", "standard_js");

        // Rust
        map.insert("rustfmt", "rustfmt");
        map.insert("cargo-fmt", "cargo_fmt");
        map.insert("clippy", "cargo_clippy");

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

        map
    }
}
