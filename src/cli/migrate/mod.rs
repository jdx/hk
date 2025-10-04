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
                format_pkl_value(glob)
            ));
        }

        if let Some(ref exclude) = step.exclude {
            output.push_str(&format!(
                "{}exclude = {}\n",
                inner_indent,
                format_pkl_value(exclude)
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

/// Try to parse a regex pattern as a simple list of literal paths
/// Returns Some(paths) if the pattern is like ^path1$|^path2$|^path3$
/// Returns None if it's a complex regex pattern
fn parse_as_path_list(pattern: &str) -> Option<Vec<String>> {
    let trimmed = pattern.trim();

    // Check if this is a (?x) verbose mode pattern
    let working_pattern = if trimmed.starts_with("(?x)") {
        trimmed[4..].trim()
    } else {
        trimmed
    };

    // Split by | and check if each part is a convertible pattern
    let parts: Vec<&str> = working_pattern.split('|').collect();

    let mut globs = Vec::new();

    for part in parts {
        let part = part.trim();

        if let Some(glob) = convert_regex_to_glob(part) {
            globs.push(glob);
        } else {
            return None; // Can't convert this part
        }
    }

    Some(globs)
}

/// Convert a simple regex pattern to a glob pattern
/// Returns Some(glob) if convertible, None otherwise
fn convert_regex_to_glob(regex: &str) -> Option<String> {
    let mut result = String::new();
    let mut chars = regex.chars().peekable();

    // Check for anchor patterns
    let has_start_anchor = regex.starts_with('^');
    let _has_end_anchor = regex.ends_with('$');

    // Skip leading ^
    if has_start_anchor {
        chars.next();
    }

    while let Some(ch) = chars.next() {
        match ch {
            // End anchor
            '$' if chars.peek().is_none() => {
                // Just skip the end anchor
            }
            // Escaped dot becomes literal dot
            '\\' => {
                if let Some(next) = chars.next() {
                    match next {
                        '.' => {
                            result.push('.');
                        }
                        _ => return None, // Other escapes not supported
                    }
                } else {
                    return None;
                }
            }
            // .* becomes **
            '.' => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume the *

                    // Check what comes after .*
                    if chars.peek() == Some(&'/') {
                        // .*/ becomes **/
                        chars.next(); // consume the /
                        result.push_str("**/");
                    } else {
                        // .* at end or before other chars becomes **
                        result.push_str("**");
                    }
                } else {
                    return None; // Single . is not a valid glob
                }
            }
            // Regular characters
            'a'..='z' | 'A'..='Z' | '0'..='9' | '/' | '-' | '_' => {
                result.push(ch);
            }
            _ => return None, // Unsupported character
        }
    }

    // If no start anchor and doesn't already start with **, add it
    if !has_start_anchor && !result.starts_with("**/") {
        result = format!("**/{}", result);
    }

    Some(result)
}

/// Format a value for Pkl - either as a List(), Regex(), or as a string
pub fn format_pkl_value(value: &str) -> String {
    // Check if this looks like a regex pattern
    if is_regex_pattern(value) {
        return format!("Regex({})", format_pkl_string(value));
    }

    // Skip path list parsing for multiline patterns to preserve formatting
    if !value.contains('\n') {
        // Try to parse as a simple path list first (only for single-line patterns)
        if let Some(paths) = parse_as_path_list(value) {
            let formatted_paths: Vec<String> = paths.iter().map(|p| format!("\"{}\"", p)).collect();
            return format!("List({})", formatted_paths.join(", "));
        }
    }

    // Otherwise format as a string
    format_pkl_string(value)
}

/// Detect if a pattern looks like a regex (vs a simple glob)
fn is_regex_pattern(pattern: &str) -> bool {
    let trimmed = pattern.trim();

    // Check for regex indicators
    trimmed.starts_with("(?x)")      // Verbose regex mode
        || trimmed.starts_with("(?i")  // Case-insensitive regex mode
        || trimmed.starts_with("(?")   // Other regex flags
        || trimmed.starts_with('^')    // Regex anchor
        || trimmed.contains("\\b")     // Word boundary
        || trimmed.contains("\\s")     // Whitespace class
        || trimmed.contains("\\d")     // Digit class
        || trimmed.contains("\\w")     // Word class
        || trimmed.contains(".*")      // Regex any sequence
        || trimmed.contains(".+")      // Regex one-or-more sequence
        || (trimmed.contains('|') && trimmed.contains('^')) // Regex alternation with anchors
}

/// Format a string value for Pkl, using custom delimiters if needed
pub fn format_pkl_string(value: &str) -> String {
    let trimmed = value.trim();

    // For regex patterns and strings with special characters, use custom delimiters
    if trimmed.contains('\\') || trimmed.contains('"') {
        if trimmed.contains('\n') {
            // Multi-line string with special chars, use multi-line custom delimiter
            format!("#\"\"\"\n{}\n\"\"\"#", trimmed)
        } else {
            // Single-line string with special chars, use single-line custom delimiter
            format!("#\"{}\"#", trimmed)
        }
    } else if trimmed.contains('\n') {
        // Multi-line string without special chars, use multi-line regular quotes
        format!("\"\"\"\n{}\n\"\"\"", trimmed)
    } else {
        // Simple string, use regular quotes
        format!("\"{}\"", trimmed)
    }
}
