use crate::Result;
use indexmap::IndexMap;

pub mod pre_commit;

/// Migrate from other hook managers to hk
#[derive(Debug, clap::Args)]
pub struct Migrate {
    #[clap(subcommand)]
    command: MigrateCommands,
}

#[derive(Debug, clap::Subcommand)]
enum MigrateCommands {
    /// Migrate from pre-commit to hk
    PreCommit(pre_commit::PreCommit),
}

impl Migrate {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            MigrateCommands::PreCommit(cmd) => cmd.run().await,
        }
    }
}

/// Intermediate representation of an hk.pkl configuration file
/// This can be serialized to PKL format
#[derive(Debug, Default)]
pub struct HkConfig {
    /// Base configuration to amend
    pub amends: String,
    /// Imports (e.g., Builtins.pkl)
    pub imports: Vec<String>,
    /// Comments at the top of the file
    pub header_comments: Vec<String>,
    /// Named step collections (e.g., "linters", "local_hooks", "custom_steps")
    pub step_collections: IndexMap<String, IndexMap<String, HkStep>>,
    /// Hook configurations
    pub hooks: IndexMap<String, HkHook>,
}

/// Represents a single step in hk configuration
#[derive(Debug, Clone)]
pub struct HkStep {
    /// The builtin to use, if any (e.g., "Builtins.yamllint")
    pub builtin: Option<String>,
    /// Comment lines before the step
    pub comments: Vec<String>,
    /// Glob pattern for files
    pub glob: Option<String>,
    /// Exclude pattern
    pub exclude: Option<String>,
    /// Prefix command (e.g., "mise x pipx:ruff@0.13.3 --")
    pub prefix: Option<String>,
    /// Check command
    pub check: Option<String>,
    /// Fix command
    pub fix: Option<String>,
    /// Shell command
    pub shell: Option<String>,
    /// Additional properties as comments (for now)
    pub properties_as_comments: Vec<String>,
}

/// Represents a hook configuration
#[derive(Debug)]
pub struct HkHook {
    /// Whether to run fix mode
    pub fix: Option<bool>,
    /// Stash strategy (e.g., "git")
    pub stash: Option<String>,
    /// Steps to include (spread references like "...linters")
    pub step_spreads: Vec<String>,
    /// Direct steps (not via spread)
    pub direct_steps: IndexMap<String, HkStep>,
}

impl HkConfig {
    /// Create a new HkConfig with specified or default amends and imports
    pub fn new(config_pkl: Option<String>, builtins_pkl: Option<String>) -> Self {
        let version = env!("CARGO_PKG_VERSION");

        let amends = config_pkl.unwrap_or_else(|| {
            format!(
                "package://github.com/jdx/hk/releases/download/v{}/hk@{}#/Config.pkl",
                version, version
            )
        });

        let imports = vec![builtins_pkl.unwrap_or_else(|| {
            format!(
                "package://github.com/jdx/hk/releases/download/v{}/hk@{}#/Builtins.pkl",
                version, version
            )
        })];

        Self {
            amends,
            imports,
            ..Default::default()
        }
    }

    /// Serialize the configuration to PKL format
    pub fn to_pkl(&self) -> String {
        let mut output = String::new();

        // Amends
        output.push_str(&format!("amends \"{}\"\n", self.amends));

        // Imports
        for import in &self.imports {
            output.push_str(&format!("import \"{}\"\n", import));
        }
        output.push('\n');

        // Header comments
        if !self.header_comments.is_empty() {
            for comment in &self.header_comments {
                output.push_str(&format!("// {}\n", comment));
            }
            output.push('\n');
        }

        // Step collections
        for (name, steps) in &self.step_collections {
            if steps.is_empty() {
                continue;
            }

            output.push_str(&format!("local {} = new Mapping<String, Step> {{\n", name));
            for (id, step) in steps {
                output.push_str(&self.format_step(id, step, 1));
            }
            output.push_str("}\n\n");
        }

        // Hooks
        if !self.hooks.is_empty() {
            output.push_str("hooks {\n");
            for (name, hook) in &self.hooks {
                output.push_str(&self.format_hook(name, hook));
            }
            output.push_str("}\n");
        }

        output
    }

    /// Format a single step
    fn format_step(&self, id: &str, step: &HkStep, indent_level: usize) -> String {
        let mut output = String::new();
        let indent = "    ".repeat(indent_level);

        // Comments
        for comment in &step.comments {
            output.push_str(&format!("{}// {}\n", indent, comment));
        }

        // Step definition
        output.push_str(&format!("{}[\"{}\"]", indent, id));

        // If it's just a builtin with no customization, use simple format
        if let Some(ref builtin) = step.builtin {
            if step.glob.is_none()
                && step.exclude.is_none()
                && step.check.is_none()
                && step.fix.is_none()
                && step.shell.is_none()
                && step.properties_as_comments.is_empty()
            {
                output.push_str(&format!(" = {}\n", builtin));
                return output;
            }

            // Builtin with customization
            output.push_str(&format!(" = ({}) {{\n", builtin));
        } else {
            // Custom step
            output.push_str(" {\n");
        }

        let inner_indent = "    ".repeat(indent_level + 1);

        // Properties
        if let Some(ref glob) = step.glob {
            output.push_str(&format!(
                "{}glob = {}\n",
                inner_indent,
                format_pkl_string(glob)
            ));
        }

        if let Some(ref exclude) = step.exclude {
            output.push_str(&format!(
                "{}exclude = {}\n",
                inner_indent,
                format_pkl_string(exclude)
            ));
        }

        if let Some(ref prefix) = step.prefix {
            output.push_str(&format!(
                "{}prefix = {}\n",
                inner_indent,
                format_pkl_string(prefix)
            ));
        }

        if let Some(ref check) = step.check {
            output.push_str(&format!(
                "{}check = {}\n",
                inner_indent,
                format_pkl_string(check)
            ));
        }

        if let Some(ref fix) = step.fix {
            output.push_str(&format!(
                "{}fix = {}\n",
                inner_indent,
                format_pkl_string(fix)
            ));
        }

        if let Some(ref shell) = step.shell {
            output.push_str(&format!(
                "{}shell = {}\n",
                inner_indent,
                format_pkl_string(shell)
            ));
        }

        // Additional properties as comments
        for comment in &step.properties_as_comments {
            output.push_str(&format!("{}// {}\n", inner_indent, comment));
        }

        output.push_str(&format!("{}}}\n", indent));
        output
    }

    /// Format a hook configuration
    fn format_hook(&self, name: &str, hook: &HkHook) -> String {
        let mut output = String::new();
        let indent = "    ";

        output.push_str(&format!("{}[\"{}\"] {{\n", indent, name));

        if let Some(fix) = hook.fix {
            output.push_str(&format!("{}    fix = {}\n", indent, fix));
        }

        if let Some(ref stash) = hook.stash {
            output.push_str(&format!("{}    stash = \"{}\"\n", indent, stash));
        }

        // Steps
        if !hook.step_spreads.is_empty() || !hook.direct_steps.is_empty() {
            output.push_str(&format!("{}    steps {{\n", indent));

            for spread in &hook.step_spreads {
                output.push_str(&format!("{}        ...{}\n", indent, spread));
            }

            for (id, step) in &hook.direct_steps {
                output.push_str(&self.format_step(id, step, 3));
            }

            output.push_str(&format!("{}    }}\n", indent));
        }

        output.push_str(&format!("{}}}\n", indent));
        output
    }
}

/// Format a string value for Pkl, using custom delimiters if needed
pub fn format_pkl_string(value: &str) -> String {
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
