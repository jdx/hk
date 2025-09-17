use codegen::{Impl, Scope, Struct};
use indexmap::IndexMap;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct SettingsRegistry {
    #[serde(flatten)]
    pub options: IndexMap<String, OptionConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OptionConfig {
    #[serde(rename = "type")]
    pub typ: String,
    pub default: Option<toml::Value>,
    pub merge: Option<String>,
    pub sources: SourcesConfig,
    pub validate: Option<ValidateConfig>,
    pub docs: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SourcesConfig {
    #[serde(default)]
    pub cli: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub git: Vec<String>,
    #[serde(default)]
    pub pkl: PklSource,
}

#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum PklSource {
    #[default]
    None,
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ValidateConfig {
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}

pub fn generate(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let settings_content = fs::read_to_string("settings.toml")?;
    let registry: SettingsRegistry = toml::from_str(&settings_content)?;

    // Generate the settings struct
    generate_settings_struct(out_dir, &registry)?;

    // Generate the CLI flags struct
    generate_cli_flags_struct(out_dir, &registry)?;

    Ok(())
}

fn generate_settings_struct(
    out_dir: &Path,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();
    scope.import("indexmap", "IndexSet");
    scope.import("std::path", "PathBuf");
    scope.raw("#[allow(dead_code)]");

    // Create the main settings struct
    let mut settings_struct = Struct::new("GeneratedSettings");
    settings_struct
        .vis("pub")
        .derive("Debug")
        .derive("Clone")
        .doc("Auto-generated settings struct from settings.toml");

    // Add fields to the struct
    for (name, opt) in &registry.options {
        let field_name = name.replace('-', "_");
        let base_type = rust_type(&opt.typ, name);

        let field_type = if is_nullable(opt) {
            format!("Option<{}>", base_type)
        } else {
            base_type
        };

        settings_struct.field(&format!("pub {}", field_name), field_type);
    }

    scope.push_struct(settings_struct);

    // Generate default implementation
    let mut default_impl = Impl::new("GeneratedSettings");
    default_impl.impl_trait("Default");

    let default_fn = default_impl.new_fn("default");
    default_fn
        .ret("Self")
        .doc("Create settings with default values");

    let mut body = vec!["Self {".to_string()];

    for (name, opt) in &registry.options {
        let field_name = name.replace('-', "_");
        let default_value = get_default_value(opt, name);
        body.push(format!("    {}: {},", field_name, default_value));
    }

    body.push("}".to_string());
    default_fn.line(body.join("\n"));

    scope.push_impl(default_impl);

    // Write the scope to file
    fs::write(out_dir.join("generated_settings.rs"), scope.to_string())?;

    Ok(())
}

fn generate_cli_flags_struct(
    out_dir: &Path,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();
    scope.import("clap", "Args");
    scope.import("std::path", "PathBuf");
    scope.raw("#[allow(dead_code)]");

    // We need to manually build this struct with documentation
    let mut struct_code = Vec::new();
    struct_code.push("#[derive(Debug, Args, Default)]".to_string());
    struct_code.push("pub struct GeneratedCliFlags {".to_string());

    // Process each option that has CLI sources
    for (name, opt) in &registry.options {
        if opt.sources.cli.is_empty() {
            continue;
        }

        let field_name = name.replace('-', "_");
        let field_type = if name == "verbose" {
            "u8".to_string()
        } else {
            format!("Option<{}>", rust_type_for_cli(&opt.typ, name))
        };

        // Add documentation
        let doc_lines = format_doc_comment(&opt.docs);
        for line in doc_lines {
            struct_code.push(format!("    /// {}", line));
        }

        // Add clap attributes
        let clap_attrs = build_clap_attributes(name, opt);
        if !clap_attrs.is_empty() {
            struct_code.push(format!("    #[clap({})]", clap_attrs.join(", ")));
        }

        struct_code.push(format!("    pub {}: {},", field_name, field_type));
    }

    struct_code.push("}".to_string());

    // Add the struct to the scope
    scope.raw(&struct_code.join("\n"));

    // Write the generated code
    fs::write(out_dir.join("generated_cli_flags.rs"), scope.to_string())?;

    Ok(())
}

fn rust_type(typ: &str, _name: &str) -> String {
    match typ {
        "bool" => "bool".to_string(),
        "int" => "usize".to_string(),
        "string" => "String".to_string(),
        "path" => "PathBuf".to_string(),
        "enum" => "String".to_string(),
        typ if typ.starts_with("list<string>") => "IndexSet<String>".to_string(),
        _ => "String".to_string(),
    }
}

fn rust_type_for_cli(typ: &str, _name: &str) -> String {
    match typ {
        "bool" => "bool".to_string(),
        "int" => "usize".to_string(),
        "string" => "String".to_string(),
        "path" => "PathBuf".to_string(),
        "enum" => "String".to_string(),
        typ if typ.starts_with("list<string>") => "Vec<String>".to_string(),
        _ => "String".to_string(),
    }
}

fn is_nullable(opt: &OptionConfig) -> bool {
    // List types default to empty if no default is specified
    if opt.typ.starts_with("list<") {
        return false;
    }
    opt.default.is_none()
}

fn get_default_value(opt: &OptionConfig, name: &str) -> String {
    if is_nullable(opt) {
        return "None".to_string();
    }

    match opt.typ.as_str() {
        "bool" => match &opt.default {
            Some(v) if v.as_bool() == Some(true) => "true",
            _ => "false",
        }
        .to_string(),
        "int" => match &opt.default {
            Some(v) => v.as_integer().unwrap_or(0).to_string(),
            None => "0".to_string(),
        },
        "string" | "enum" => match &opt.default {
            Some(v) => format!("\"{}\".to_string()", v.as_str().unwrap_or("")),
            None => "String::new()".to_string(),
        },
        "path" => match &opt.default {
            Some(v) => format!("PathBuf::from(\"{}\")", v.as_str().unwrap_or("")),
            None => "PathBuf::new()".to_string(),
        },
        typ if typ.starts_with("list<") => match &opt.default {
            Some(toml::Value::Array(vals)) if !vals.is_empty() => {
                let items = vals
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| format!("\"{s}\".to_string()"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("IndexSet::from([{}])", items)
            }
            _ => "IndexSet::new()".to_string(),
        },
        _ => {
            eprintln!("Warning: Unknown type '{}' for field '{}'", opt.typ, name);
            "Default::default()".to_string()
        }
    }
}

fn format_doc_comment(docs: &str) -> Vec<String> {
    // Remove backticks and single quotes to avoid syntax issues
    docs.replace('`', "")
        .replace('\'', "")
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn build_clap_attributes(name: &str, opt: &OptionConfig) -> Vec<String> {
    let mut attrs = Vec::new();

    // Collect long and short flags
    let mut long_flags = Vec::new();
    let mut short_flags = Vec::new();

    for flag in &opt.sources.cli {
        if flag.starts_with("--") {
            let long = flag.strip_prefix("--").unwrap();
            long_flags.push(long);
        } else if flag.starts_with('-') && flag.len() == 2 {
            short_flags.push(flag.chars().nth(1).unwrap());
        }
    }

    // For boolean options with both positive and negative flags,
    // we only use the positive form. Clap will automatically handle
    // --no-<flag> for boolean options when using Option<bool>
    if opt.typ == "bool" {
        // Find the primary (non-negated) flag
        if let Some(primary) = long_flags.iter().find(|&&f| !f.starts_with("no-")) {
            attrs.push(format!("long = \"{}\"", primary));
            // Check if there's also a negation flag - if so, we need to enable overrides_with
            if long_flags.iter().any(|&f| f.starts_with("no-")) {
                // For Option<bool>, clap automatically handles --no-<flag>
                // We just need to specify the primary flag
            }
        } else {
            // Only negation flags present (unusual case)
            for long in &long_flags {
                attrs.push(format!("long = \"{}\"", long));
            }
        }
    } else {
        // Non-boolean options - for multiple long flags, we can only use the first one
        // Clap doesn't support multiple long attributes in a single #[clap()] attribute
        // Note: This means --exclude-glob won't work in the generated CLI flags,
        // but it's still supported through the manual CLI parsing in hook_options.rs
        if let Some(first_long) = long_flags.first() {
            attrs.push(format!("long = \"{}\"", first_long));
        }
    }

    // Add short flags
    for short in short_flags {
        attrs.push(format!("short = '{}'", short));
    }

    // Special handling for verbose (count flag)
    if name == "verbose" {
        attrs.push("action = clap::ArgAction::Count".to_string());
    }

    attrs
}
