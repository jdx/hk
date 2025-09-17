use codegen::Scope;
use indexmap::IndexMap;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct SettingsRegistry {
    pub option: IndexMap<String, OptionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct OptionConfig {
    #[serde(rename = "type")]
    pub typ: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub default: Option<toml::Value>,
    #[serde(default = "default_merge")]
    #[allow(dead_code)]
    pub merge: String,
    pub sources: Sources,
    pub docs: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub validate: Validate,
}

fn default_merge() -> String {
    "replace".to_string()
}

#[derive(Debug, Default, Deserialize)]
pub struct Sources {
    #[serde(default)]
    #[allow(dead_code)]
    pub pkl: SourceDef,
    #[serde(default)]
    #[allow(dead_code)]
    pub env: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub git: Vec<String>,
    #[serde(default)]
    pub cli: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum SourceDef {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for SourceDef {
    fn default() -> Self {
        SourceDef::Multiple(vec![])
    }
}

impl SourceDef {
    #[allow(dead_code)]
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            SourceDef::Single(s) => vec![s.clone()],
            SourceDef::Multiple(v) => v.clone(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct Validate {
    #[serde(rename = "enum", default)]
    pub enum_values: Vec<String>,
}

pub fn generate(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let settings_toml = fs::read_to_string("settings.toml")?;
    let registry: SettingsRegistry = toml::from_str(&settings_toml)?;

    // Generate settings struct
    generate_settings_module(&registry, out_dir)?;

    // Generate CLI flags
    generate_cli_flags(&registry, out_dir)?;

    Ok(())
}

fn generate_settings_module(
    registry: &SettingsRegistry,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();

    // Add imports
    scope.raw("#[allow(dead_code)]");
    scope.import("indexmap", "IndexSet");
    scope.import("std::num", "NonZero");
    scope.import("std::path", "PathBuf");

    // Generate the Settings struct
    let settings_struct = scope
        .new_struct("GeneratedSettings")
        .derive("Debug")
        .derive("Clone")
        .vis("pub");

    for (name, opt) in &registry.option {
        let field_name = name.replace('-', "_");
        let base_type = rust_type(&opt.typ, name);
        let field_type = if is_nullable(opt) {
            format!("Option<{}>", base_type)
        } else {
            base_type
        };
        settings_struct
            .field(&format!("pub {}", field_name), &field_type)
            .doc(&opt.docs);
    }

    // Write to file
    let output = scope.to_string();
    fs::write(out_dir.join("generated_settings.rs"), output)?;

    Ok(())
}

fn generate_cli_flags(
    registry: &SettingsRegistry,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();

    scope.raw("#[allow(dead_code)]");
    scope.import("clap", "Args");
    scope.import("std::path", "PathBuf");

    let cli_struct = scope
        .new_struct("GeneratedCliFlags")
        .derive("Debug")
        .derive("Args")
        .derive("Default")
        .vis("pub");

    for (name, opt) in &registry.option {
        if opt.sources.cli.is_empty() {
            continue;
        }

        let field_name = name.replace('-', "_");
        let field_type = clap_type(&opt.typ);

        // Build clap attributes
        let mut attrs = Vec::new();
        for cli_flag in &opt.sources.cli {
            if cli_flag.starts_with("--") {
                let flag = cli_flag.trim_start_matches("--");
                if flag == name {
                    attrs.push("long".to_string());
                } else {
                    attrs.push(format!("long = \"{}\"", flag));
                }
            } else if cli_flag.starts_with("-") && cli_flag.len() == 2 {
                let flag = cli_flag.trim_start_matches("-");
                attrs.push(format!("short = '{}'", flag.chars().next().unwrap()));
            }
        }

        if name == "verbose" {
            attrs.push("action = clap::ArgAction::Count".to_string());
        }

        // Add field with attributes
        let field_definition = if attrs.is_empty() {
            format!("/// {}\npub {}", opt.docs, field_name)
        } else {
            format!(
                "/// {}\n#[clap({})]\npub {}",
                opt.docs,
                attrs.join(", "),
                field_name
            )
        };

        let field_type_str = if name == "verbose" {
            "u8".to_string()
        } else {
            format!("Option<{}>", field_type)
        };

        cli_struct.field(&field_definition, &field_type_str);
    }

    fs::write(out_dir.join("generated_cli_flags.rs"), scope.to_string())?;
    Ok(())
}

fn rust_type(typ: &str, name: &str) -> String {
    match typ {
        "bool" => "bool".to_string(),
        "int" => {
            if name == "jobs" || name == "verbose" {
                "NonZero<usize>".to_string()
            } else {
                "usize".to_string()
            }
        }
        "string" => "String".to_string(),
        "path" => "PathBuf".to_string(),
        "enum" => "String".to_string(),
        typ if typ.starts_with("list<string>") => "IndexSet<String>".to_string(),
        _ => "String".to_string(),
    }
}

fn is_nullable(opt: &OptionConfig) -> bool {
    opt.default.is_none()
}

fn clap_type(typ: &str) -> String {
    match typ {
        "bool" => "bool".to_string(),
        "int" => "usize".to_string(),
        "string" => "String".to_string(),
        "path" => "PathBuf".to_string(),
        "enum" => "String".to_string(),
        typ if typ.starts_with("list<") => "Vec<String>".to_string(),
        _ => "String".to_string(),
    }
}
