use codegen::{Enum, Function, Impl, Scope, Struct, Variant};
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

    // Generate docs from the centralized settings registry
    generate_configuration_docs(&registry)?;

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

    // Define SettingValue enum
    let mut setting_value = Enum::new("SettingValue");
    setting_value.vis("pub").derive("Clone").derive("Debug");

    {
        let mut v = Variant::new("Bool");
        v.tuple("bool");
        setting_value.push_variant(v);
    }
    {
        let mut v = Variant::new("Usize");
        v.tuple("usize");
        setting_value.push_variant(v);
    }
    {
        let mut v = Variant::new("U8");
        v.tuple("u8");
        setting_value.push_variant(v);
    }
    {
        let mut v = Variant::new("String");
        v.tuple("String");
        setting_value.push_variant(v);
    }
    {
        let mut v = Variant::new("Path");
        v.tuple("PathBuf");
        setting_value.push_variant(v);
    }
    {
        let mut v = Variant::new("StringList");
        v.tuple("IndexSet<String>");
        setting_value.push_variant(v);
    }
    scope.push_enum(setting_value);

    // Type alias for map of settings values
    scope.raw("pub type SourceMap = IndexMap<&'static str, SettingValue>;");

    // Provenance tracking types
    let mut setting_source = Enum::new("SettingSource");
    setting_source.vis("pub").derive("Clone").derive("Debug");
    setting_source.push_variant(Variant::new("Defaults"));
    setting_source.push_variant(Variant::new("Env"));
    setting_source.push_variant(Variant::new("Git"));
    setting_source.push_variant(Variant::new("Pkl"));
    setting_source.push_variant(Variant::new("Cli"));
    scope.push_enum(setting_source);

    let mut source_info_entry = Struct::new("SourceInfoEntry");
    source_info_entry
        .vis("pub")
        .derive("Clone")
        .derive("Debug")
        .derive("Default")
        .field("pub last", "Option<SettingSource>")
        .field(
            "pub list_items",
            "Option<IndexMap<String, Vec<SettingSource>>>",
        );
    scope.push_struct(source_info_entry);

    // Type alias for per-field provenance
    scope.raw("pub type SourceInfoMap = IndexMap<&'static str, SourceInfoEntry>;");

    // Only types are generated; merge logic implemented in src/settings.rs
    // Write the scope to file
    fs::write(
        out_dir.join("generated_settings_merge.rs"),
        scope.to_string(),
    )?;
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

    // Build a function to construct SETTINGS_META using typed codegen
    let mut build_fn = Function::new("build_settings_meta");
    build_fn.ret("IndexMap<&'static str, SettingMeta>");
    build_fn.line("let mut m: IndexMap<&'static str, SettingMeta> = IndexMap::new();");
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
        build_fn.line(format!("m.insert({:?}, SettingMeta {{", name));
        build_fn.line(format!("    typ: {:?},", opt.typ));
        build_fn.line(format!("    default_value: {},", default_value));
        build_fn.line(format!("    merge: {},", merge));
        build_fn.line("    sources: SettingSourcesMeta {");
        build_fn.line(format!("        cli: {},", cli_sources));
        build_fn.line(format!("        env: {},", env_sources));
        build_fn.line(format!("        git: {},", git_sources));
        build_fn.line(format!("        pkl: {},", pkl_sources));
        build_fn.line("    },");
        build_fn.line("});");
    }
    build_fn.line("m");
    scope.push_fn(build_fn);

    // Define the static using the builder function
    scope.raw("pub static SETTINGS_META: Lazy<IndexMap<&'static str, SettingMeta>> = Lazy::new(build_settings_meta);");

    // Write the scope to file
    fs::write(
        out_dir.join("generated_settings_meta.rs"),
        scope.to_string(),
    )?;

    Ok(())
}

fn generate_configuration_docs(
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build a concise markdown section summarizing settings
    // We either replace content between markers in docs/configuration.md
    // or append to the end if markers are not present.
    const START_MARKER: &str = "<!-- BEGIN: AUTO-GENERATED SETTINGS -->";
    const END_MARKER: &str = "<!-- END: AUTO-GENERATED SETTINGS -->";

    let mut md = String::new();
    md.push_str(START_MARKER);
    md.push_str("\n\n");

    // Sorted for stable output
    let mut keys: Vec<_> = registry.options.keys().cloned().collect();
    keys.sort();

    md.push('\n');
    // Include per-setting docs as collapsible sections
    for key in &keys {
        let opt = registry.options.get(key).unwrap();
        md.push_str(&format!("### `{}`\n\n", key.replace('_', "-")));
        // Metadata: unordered list with type, default (if any), and sources
        md.push_str(&format!("- Type: `{}`\n", opt.typ));
        if let Some(default) = &opt.default {
            md.push_str(&format!("- Default: `{}`\n", default));
        }
        // Sources (if any)
        let mut any_sources = false;
        let mut sources_block = String::new();
        if !opt.sources.cli.is_empty() {
            any_sources = true;
            let items = opt
                .sources
                .cli
                .iter()
                .map(|s| format!("`{}`", s))
                .collect::<Vec<_>>()
                .join(", ");
            sources_block.push_str(&format!("- CLI: {}\n", items));
        }
        if !opt.sources.env.is_empty() {
            any_sources = true;
            let items = opt
                .sources
                .env
                .iter()
                .map(|s| format!("`{}`", s))
                .collect::<Vec<_>>()
                .join(", ");
            sources_block.push_str(&format!("- ENV: {}\n", items));
        }
        if !opt.sources.git.is_empty() {
            any_sources = true;
            let items = opt
                .sources
                .git
                .iter()
                .map(|s| format!("`{}`", s))
                .collect::<Vec<_>>()
                .join(", ");
            sources_block.push_str(&format!("- Git: {}\n", items));
        }
        match &opt.sources.pkl {
            PklSource::None => {}
            PklSource::Single(s) => {
                if !s.starts_with("defaults.") {
                    any_sources = true;
                    sources_block.push_str(&format!("- Pkl: `{}`\n", s));
                }
            }
            PklSource::Multiple(v) => {
                let filtered: Vec<&String> =
                    v.iter().filter(|s| !s.starts_with("defaults.")).collect();
                if !filtered.is_empty() {
                    any_sources = true;
                    let items = filtered
                        .iter()
                        .map(|s| format!("`{}`", s))
                        .collect::<Vec<_>>()
                        .join(", ");
                    sources_block.push_str(&format!("- Pkl: {}\n", items));
                }
            }
        }
        if any_sources {
            md.push_str("- Sources:\n");
            // indent nested bullets
            let nested = sources_block
                .lines()
                .map(|l| format!("  {}", l))
                .collect::<Vec<_>>()
                .join("\n");
            md.push_str(&nested);
            md.push('\n');
        }
        md.push('\n');
        md.push_str(opt.docs.trim());
        md.push_str("\n\n");
    }
    md.push_str(END_MARKER);

    // Read docs/configuration.md and replace/append
    let path = "docs/configuration.md";
    let existing = fs::read_to_string(path).unwrap_or_else(|_| String::new());
    let s = existing.find(START_MARKER).unwrap();
    let e = existing.find(END_MARKER).unwrap();
    let e_end = e + END_MARKER.len();
    let mut new_content = String::new();
    new_content.push_str(&existing[..s]);
    new_content.push_str(&md);
    new_content.push_str(&existing[e_end..]);

    // Ensure exactly one newline at the end of the file
    new_content = new_content.trim_end().to_string();
    new_content.push('\n');

    fs::write(path, new_content)?;

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
