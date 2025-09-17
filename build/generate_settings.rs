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
pub struct OptionConfig {
    #[serde(rename = "type")]
    pub typ: String,
    pub default: Option<toml::Value>,
    pub merge: Option<String>,
    pub sources: SourcesConfig,
    pub validate: Option<ValidateConfig>,
    pub docs: String,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub deprecated: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
}

#[derive(Debug, Deserialize)]
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
pub enum PklSource {
    #[default]
    None,
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct ValidateConfig {
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}

pub fn generate(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let settings_content = fs::read_to_string("settings.toml")?;
    let registry: SettingsRegistry = toml::from_str(&settings_content)?;

    // Generate the settings struct with documentation
    generate_settings_struct(out_dir, &registry)?;

    // Also generate merge/types
    generate_settings_merge(out_dir, &registry)?;

    // Generate the settings meta
    generate_settings_meta(out_dir, &registry)?;

    // Generate JSON Schema for external tooling
    generate_json_schema(&registry)?;

    Ok(())
}

fn generate_settings_struct(
    out_dir: &Path,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();
    scope.import("indexmap", "IndexSet");
    scope.import("std::path", "PathBuf");

    // Create the main settings struct
    let mut settings_struct = Struct::new("Settings");
    settings_struct
        .vis("pub")
        .derive("Debug")
        .derive("Clone")
        .derive("serde::Serialize")
        .derive("serde::Deserialize")
        .doc("Auto-generated settings struct from settings.toml");

    // Add fields to the struct with documentation
    for (name, opt) in &registry.options {
        let field_name = name.replace('-', "_");
        let base_type = rust_type(&opt.typ);

        let field_type = if is_nullable(opt) {
            format!("Option<{}>", base_type)
        } else {
            base_type
        };

        // Build comprehensive documentation
        let mut doc_lines = vec![];

        // Main documentation
        doc_lines.extend(opt.docs.lines().map(|l| l.to_string()));

        // Add deprecation notice if present
        if let Some(deprecated) = &opt.deprecated {
            doc_lines.push(String::new());
            doc_lines.push("# Deprecated".to_string());
            doc_lines.push(deprecated.clone());
        }

        // Add since version if present
        if let Some(since) = &opt.since {
            doc_lines.push(String::new());
            doc_lines.push(format!("Since: v{}", since));
        }

        // Add examples if present
        if !opt.examples.is_empty() {
            doc_lines.push(String::new());
            doc_lines.push("# Examples".to_string());
            for example in &opt.examples {
                doc_lines.push(format!("- {}", example));
            }
        }

        // Add default value info
        if let Some(default) = &opt.default {
            doc_lines.push(String::new());
            doc_lines.push(format!("Default: `{}`", default));
        }

        // Add type info
        doc_lines.push(String::new());
        doc_lines.push(format!("Type: `{}`", opt.typ));

        // Add source info
        let mut sources = vec![];
        if !opt.sources.cli.is_empty() {
            sources.push(format!("CLI: {}", opt.sources.cli.join(", ")));
        }
        if !opt.sources.env.is_empty() {
            sources.push(format!("ENV: {}", opt.sources.env.join(", ")));
        }
        if !opt.sources.git.is_empty() {
            sources.push(format!("Git: {}", opt.sources.git.join(", ")));
        }

        if !sources.is_empty() {
            doc_lines.push(String::new());
            doc_lines.push("Configuration sources:".to_string());
            for source in sources {
                doc_lines.push(format!("- {}", source));
            }
        }

        // Create a field with documentation
        let mut field = codegen::Field::new(&format!("pub {}", field_name), field_type);
        if !doc_lines.is_empty() {
            field.doc(doc_lines.join("\n"));
        }
        settings_struct.push_field(field);
    }

    scope.push_struct(settings_struct);

    // Generate default implementation
    let mut default_impl = Impl::new("Settings");
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

fn generate_settings_merge(
    out_dir: &Path,
    _registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();
    scope.import("indexmap", "IndexMap");
    scope.import("indexmap", "IndexSet");
    scope.import("std::path", "PathBuf");

    // Define SettingValue and SourceMap types
    scope.raw("#[derive(Clone)]\npub enum SettingValue {\n    Bool(bool),\n    Usize(usize),\n    U8(u8),\n    String(String),\n    Path(PathBuf),\n    StringList(IndexSet<String>),\n}");
    scope.raw("pub type SourceMap = IndexMap<&'static str, SettingValue>;");

    // Provenance tracking types
    scope.raw("#[derive(Clone, Debug)]\npub enum SettingSource { Defaults, Env, Git, Pkl, Cli }");
    scope.raw("#[derive(Clone, Debug, Default)]\npub struct SourceInfoEntry {\n    pub last: Option<SettingSource>,\n    pub list_items: Option<IndexMap<String, Vec<SettingSource>>>,\n}");
    scope.raw("pub type SourceInfoMap = IndexMap<&'static str, SourceInfoEntry>;");

    // Only types are generated; merge logic implemented in src/settings.rs
    // Write the scope to file
    fs::write(out_dir.join("generated_settings_merge.rs"), scope.to_string())?;
    Ok(())
}

fn generate_settings_meta(
    out_dir: &Path,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();
    scope.import("indexmap", "IndexMap");
    scope.import("once_cell::sync", "Lazy");

    // Generate SettingMeta struct
    let mut setting_meta_struct = Struct::new("SettingMeta");
    setting_meta_struct
        .vis("pub")
        .derive("Debug")
        .derive("Clone")
        .field("pub typ", "&'static str")
        .field("pub default_value", "Option<&'static str>")
        .field("pub merge", "Option<&'static str>")
        .field("pub sources", "SettingSourcesMeta");

    scope.push_struct(setting_meta_struct);

    // Generate SettingSourcesMeta struct
    let mut sources_meta_struct = Struct::new("SettingSourcesMeta");
    sources_meta_struct
        .vis("pub")
        .derive("Debug")
        .derive("Clone")
        .field("pub cli", "&'static [&'static str]")
        .field("pub env", "&'static [&'static str]")
        .field("pub git", "&'static [&'static str]")
        .field("pub pkl", "&'static [&'static str]");

    scope.push_struct(sources_meta_struct);

    // Generate the static SETTINGS_META
    let mut static_entries = Vec::new();
    for (name, opt) in &registry.options {
        let cli_sources = format_string_array(&opt.sources.cli);
        let env_sources = format_string_array(&opt.sources.env);
        let git_sources = format_string_array(&opt.sources.git);

        let pkl_sources = match &opt.sources.pkl {
            PklSource::None => "&[]".to_string(),
            PklSource::Single(s) => format!("&[{:?}]", s),
            PklSource::Multiple(v) => format_string_array(v),
        };

        let default_value = match &opt.default {
            Some(v) => format!("Some({:?})", v.as_str().unwrap_or(&v.to_string())),
            None => "None".to_string(),
        };

        let merge = match &opt.merge {
            Some(m) => format!("Some({:?})", m),
            None => "None".to_string(),
        };

        static_entries.push(format!(
            "        ({:?}, SettingMeta {{\n            typ: {:?},\n            default_value: {},\n            merge: {},\n            sources: SettingSourcesMeta {{\n                cli: {},\n                env: {},\n                git: {},\n                pkl: {},\n            }},\n        }})",
            name, opt.typ, default_value, merge, cli_sources, env_sources, git_sources, pkl_sources
        ));
    }

    // Create the static variable using raw code since codegen doesn't have great support for complex statics
    scope.raw(format!(
        "pub static SETTINGS_META: Lazy<IndexMap<&'static str, SettingMeta>> =\n    Lazy::new(|| {{\n        IndexMap::from([\n{}\n        ])\n    }});",
        static_entries.join(",\n")
    ));

    // Write the scope to file
    fs::write(out_dir.join("generated_settings_meta.rs"), scope.to_string())?;

    Ok(())
}

fn format_string_array(strings: &[String]) -> String {
    if strings.is_empty() {
        "&[]".to_string()
    } else {
        let items = strings
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>()
            .join(", ");
        format!("&[{}]", items)
    }
}

fn generate_json_schema(registry: &SettingsRegistry) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::{json};

    let mut properties = serde_json::Map::new();

    for (name, _opt) in &registry.options {
        let field_name = name.replace('-', "_");

        // Schema for a single option definition object in settings.toml
        let option_schema = json!({
            "type": "object",
            "required": ["type", "sources", "docs"],
            "additionalProperties": false,
            "properties": {
                "type": { "type": "string", "enum": ["bool","usize","u8","string","path","enum","list<string>"] },
                "default": {},
                "merge": { "type": "string", "enum": ["union","replace"] },
                "sources": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "cli": { "type": "array", "items": {"type": "string"} },
                        "env": { "type": "array", "items": {"type": "string"} },
                        "git": { "type": "array", "items": {"type": "string"} },
                        "pkl": { "type": "array", "items": {"type": "string"} }
                    }
                },
                "validate": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "enum": { "type": "array", "items": {"type": "string"} }
                    }
                },
                "docs": { "type": "string" },
                "examples": { "type": "array", "items": {"type": "string"} },
                "deprecated": { "type": "string" },
                "since": { "type": "string" }
            }
        });

        properties.insert(field_name, option_schema);
        // Each option name is required in registry? keep not required to allow partials; skip.
    }

    // Build the complete schema
    let schema = json!({
        "$schema": "https://json-schema.org/draft-07/schema#",
        "title": "HK Settings Registry",
        "description": "Schema for settings.toml (settings registry)",
        "type": "object",
        "properties": properties,
        "required": [],
        "additionalProperties": false,
        "$comment": "This schema is auto-generated from settings.toml by build/generate_settings.rs"
    });

    // Write the schema to file
    let schema_str = serde_json::to_string_pretty(&schema)?;
    fs::write("settings-schema.json", schema_str)?;

    println!("Generated settings-schema.json");

    Ok(())
}

fn rust_type(typ: &str) -> String {
    match typ {
        "bool" => "bool".to_string(),
        "usize" => "usize".to_string(),
        "u8" => "u8".to_string(),
        "string" => "String".to_string(),
        "path" => "PathBuf".to_string(),
        "enum" => "String".to_string(),
        typ if typ.starts_with("list<string>") => "IndexSet<String>".to_string(),
        _ => "String".to_string(),
    }
}

fn is_nullable(opt: &OptionConfig) -> bool {
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
        "usize" | "u8" => match &opt.default {
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
