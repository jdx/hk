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
    pub default: toml::Value,
    pub merge: String,
    pub sources: Sources,
    pub docs: String,
    #[serde(default)]
    pub validate: Validate,
}

#[derive(Debug, Default, Deserialize)]
pub struct Sources {
    #[serde(default)]
    pub pkl: SourceDef,
    #[serde(default)]
    pub env: Vec<String>,
    #[serde(default)]
    pub git: Vec<String>,
    #[serde(default)]
    pub cli: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
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
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            SourceDef::Single(s) => vec![s.clone()],
            SourceDef::Multiple(v) => v.clone(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Validate {
    #[serde(rename = "enum", default)]
    pub enum_values: Vec<String>,
}

pub fn generate(out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let settings_toml = fs::read_to_string("settings.toml")?;
    let registry: SettingsRegistry = toml::from_str(&settings_toml)?;

    // Generate settings struct and builder
    generate_settings_module(&registry, out_dir)?;

    // Generate CLI flags
    generate_cli_flags(&registry, out_dir)?;

    // Generate git config keys
    generate_git_keys(&registry, out_dir)?;

    Ok(())
}

fn generate_settings_module(
    registry: &SettingsRegistry,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();

    // Add imports
    scope.raw("#[allow(dead_code)]");
    scope.raw("#[allow(unused_imports)]");
    scope.import("indexmap", "IndexSet");
    scope.import("std::collections", "HashMap");
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
        let field_type = rust_type(&opt.typ, name);
        settings_struct
            .field(&format!("pub {}", field_name), &field_type)
            .doc(&opt.docs);
    }

    // Generate SettingsSources struct
    scope
        .new_struct("SettingsSources")
        .derive("Debug")
        .derive("Clone")
        .vis("pub")
        .doc("Map of field name to source description")
        .field("pub sources", "HashMap<String, String>");

    // Generate the builder implementation
    generate_builder_impl(&mut scope, registry)?;

    // Write to file
    let output = scope.to_string();
    fs::write(out_dir.join("generated_settings.rs"), output)?;

    Ok(())
}

fn generate_builder_impl(
    scope: &mut Scope,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    // Add the builder struct
    scope
        .new_struct("SettingsBuilder")
        .vis("pub")
        .field("defaults", "LayerValues")
        .field("pkl", "LayerValues")
        .field("git", "LayerValues")
        .field("env", "LayerValues")
        .field("cli", "LayerValues")
        .field("sources", "HashMap<String, String>");

    // LayerValues struct
    let layer_struct = scope.new_struct("LayerValues").derive("Default");

    for (name, _) in &registry.option {
        let field_name = name.replace('-', "_");
        layer_struct.field(&field_name, "Option<toml::Value>");
    }

    // Generate impl block for SettingsBuilder
    let impl_block = scope.new_impl("SettingsBuilder");

    // new() method
    let new_fn = impl_block.new_fn("new").vis("pub").ret("Self");

    new_fn.line("let mut builder = Self {");
    new_fn.line("    defaults: LayerValues::default(),");
    new_fn.line("    pkl: LayerValues::default(),");
    new_fn.line("    git: LayerValues::default(),");
    new_fn.line("    env: LayerValues::default(),");
    new_fn.line("    cli: LayerValues::default(),");
    new_fn.line("    sources: HashMap::new(),");
    new_fn.line("};");
    new_fn.line("");
    new_fn.line("// Set built-in defaults");

    for (name, opt) in &registry.option {
        let field_name = name.replace('-', "_");
        let default_str = match &opt.default {
            toml::Value::Boolean(b) => format!("toml::Value::Boolean({})", b),
            toml::Value::Integer(i) => format!("toml::Value::Integer({})", i),
            toml::Value::Float(f) => format!("toml::Value::Float({})", f),
            toml::Value::String(s) => format!("toml::Value::String({:?}.to_string())", s),
            toml::Value::Array(arr) => {
                if arr.is_empty() {
                    "toml::Value::Array(vec![])".to_string()
                } else {
                    let items = arr
                        .iter()
                        .map(|v| match v {
                            toml::Value::String(s) => {
                                format!("toml::Value::String({:?}.to_string())", s)
                            }
                            _ => format!("toml::Value::from({:?})", v),
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("toml::Value::Array(vec![{}])", items)
                }
            }
            _ => format!("toml::Value::from({:?})", opt.default),
        };
        new_fn.line(format!(
            "builder.defaults.{} = Some({});",
            field_name, default_str
        ));
    }
    new_fn.line("builder");

    // add_env_source() method
    generate_env_source_method(impl_block, registry)?;

    // add_git_source() method
    generate_git_source_method(impl_block, registry)?;

    // add_cli_source() method - stub for now
    impl_block
        .new_fn("add_cli_source")
        .vis("pub")
        .arg_mut_self()
        .arg("_args", "&clap::ArgMatches")
        .line("// TODO: Extract CLI args");

    // add_pkl_source() method - stub for now
    impl_block
        .new_fn("add_pkl_source")
        .vis("pub")
        .arg_mut_self()
        .arg("_config", "&crate::config::Config")
        .arg("_user_config", "&crate::config::UserConfig")
        .line("// TODO: Extract PKL config");

    // build() method
    generate_build_method(impl_block, registry)?;

    // Add helper function for git config
    scope.raw(
        r#"
fn read_string_list(config: &git2::Config, key: &str) -> Result<IndexSet<String>, git2::Error> {
    let mut result = IndexSet::new();
    match config.multivar(key, None) {
        Ok(mut entries) => {
            while let Some(entry) = entries.next() {
                if let Some(value) = entry?.value() {
                    for item in value.split(',').map(|s| s.trim()) {
                        if !item.is_empty() {
                            result.insert(item.to_string());
                        }
                    }
                }
            }
        }
        Err(_) => {
            if let Ok(value) = config.get_string(key) {
                for item in value.split(',').map(|s| s.trim()) {
                    if !item.is_empty() {
                        result.insert(item.to_string());
                    }
                }
            }
        }
    }
    Ok(result)
}"#,
    );

    Ok(())
}

fn generate_env_source_method(
    impl_block: &mut codegen::Impl,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let method = impl_block
        .new_fn("add_env_source")
        .vis("pub")
        .arg_mut_self();

    for (name, opt) in &registry.option {
        let field_name = name.replace('-', "_");
        for env_var in &opt.sources.env {
            method.line(format!(
                "if let Ok(val) = std::env::var(\"{}\") {{",
                env_var
            ));

            match opt.typ.as_str() {
                "bool" => {
                    method.line("    let b = val == \"true\" || val == \"1\";");
                    method.line(format!(
                        "    self.env.{} = Some(toml::Value::Boolean(b));",
                        field_name
                    ));
                    method.line(format!(
                        "    self.sources.insert(\"{}\".to_string(), \"env:{}\".to_string());",
                        field_name, env_var
                    ));
                }
                "int" => {
                    method.line("    if let Ok(i) = val.parse::<i64>() {");
                    method.line(format!(
                        "        self.env.{} = Some(toml::Value::Integer(i));",
                        field_name
                    ));
                    method.line(format!(
                        "        self.sources.insert(\"{}\".to_string(), \"env:{}\".to_string());",
                        field_name, env_var
                    ));
                    method.line("    }");
                }
                typ if typ.starts_with("list<") => {
                    method.line("    let items: Vec<toml::Value> = val.split(',').map(|s| toml::Value::String(s.trim().to_string())).collect();");
                    method.line(format!(
                        "    self.env.{} = Some(toml::Value::Array(items));",
                        field_name
                    ));
                    method.line(format!(
                        "    self.sources.insert(\"{}\".to_string(), \"env:{}\".to_string());",
                        field_name, env_var
                    ));
                }
                _ => {
                    method.line(format!(
                        "    self.env.{} = Some(toml::Value::String(val));",
                        field_name
                    ));
                    method.line(format!(
                        "    self.sources.insert(\"{}\".to_string(), \"env:{}\".to_string());",
                        field_name, env_var
                    ));
                }
            }

            method.line("}");
        }
    }

    Ok(())
}

fn generate_git_source_method(
    impl_block: &mut codegen::Impl,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let method = impl_block
        .new_fn("add_git_source")
        .vis("pub")
        .arg_mut_self()
        .arg("config", "&git2::Config")
        .ret("Result<(), git2::Error>");

    for (name, opt) in &registry.option {
        let field_name = name.replace('-', "_");
        for git_key in &opt.sources.git {
            match opt.typ.as_str() {
                "bool" => {
                    method.line(format!(
                        "if let Ok(val) = config.get_bool(\"{}\") {{",
                        git_key
                    ));
                    method.line(format!(
                        "    self.git.{} = Some(toml::Value::Boolean(val));",
                        field_name
                    ));
                    method.line(format!(
                        "    self.sources.insert(\"{}\".to_string(), \"git:{}\".to_string());",
                        field_name, git_key
                    ));
                    method.line("}");
                }
                "int" => {
                    method.line(format!(
                        "if let Ok(val) = config.get_i32(\"{}\") {{",
                        git_key
                    ));
                    method.line(format!(
                        "    self.git.{} = Some(toml::Value::Integer(val as i64));",
                        field_name
                    ));
                    method.line(format!(
                        "    self.sources.insert(\"{}\".to_string(), \"git:{}\".to_string());",
                        field_name, git_key
                    ));
                    method.line("}");
                }
                "string" | "path" | "enum" => {
                    method.line(format!(
                        "if let Ok(val) = config.get_string(\"{}\") {{",
                        git_key
                    ));
                    method.line(format!(
                        "    self.git.{} = Some(toml::Value::String(val));",
                        field_name
                    ));
                    method.line(format!(
                        "    self.sources.insert(\"{}\".to_string(), \"git:{}\".to_string());",
                        field_name, git_key
                    ));
                    method.line("}");
                }
                typ if typ.starts_with("list<") => {
                    method.line(format!(
                        "if let Ok(vals) = read_string_list(config, \"{}\") {{",
                        git_key
                    ));
                    method.line(
                        "    let array = vals.into_iter().map(toml::Value::String).collect();",
                    );
                    method.line(format!(
                        "    self.git.{} = Some(toml::Value::Array(array));",
                        field_name
                    ));
                    method.line(format!(
                        "    self.sources.insert(\"{}\".to_string(), \"git:{}\".to_string());",
                        field_name, git_key
                    ));
                    method.line("}");
                }
                _ => {}
            }
        }
    }

    method.line("Ok(())");
    Ok(())
}

fn generate_build_method(
    impl_block: &mut codegen::Impl,
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
    let method = impl_block
        .new_fn("build")
        .vis("pub")
        .arg_self()
        .ret("(GeneratedSettings, SettingsSources)");

    method.line("let settings = GeneratedSettings {");

    for (name, opt) in &registry.option {
        let field_name = name.replace('-', "_");

        method.line(format!("    {}: {{", field_name));

        if opt.merge == "union" {
            // Union merge for lists
            method.line("        let mut result = IndexSet::new();");

            for layer in ["defaults", "pkl", "git", "env", "cli"] {
                method.line(format!(
                    "        if let Some(toml::Value::Array(arr)) = &self.{}.{} {{",
                    layer, field_name
                ));
                method.line("            for val in arr {");
                method.line("                if let toml::Value::String(s) = val {");
                method.line("                    result.insert(s.clone());");
                method.line("                }");
                method.line("            }");
                method.line("        }");
            }

            if name == "display_skip_reasons" && opt.default == toml::Value::Array(vec![]) {
                method.line("        if result.is_empty() {");
                method.line("            result.insert(\"profile-not-enabled\".to_string());");
                method.line("        }");
            }

            method.line("        result");
        } else {
            // Replace merge
            method.line(format!("        let val = self.cli.{}", field_name));
            method.line(format!("            .or(self.env.{})", field_name));
            method.line(format!("            .or(self.git.{})", field_name));
            method.line(format!("            .or(self.pkl.{})", field_name));
            method.line(format!("            .or(self.defaults.{});", field_name));

            // Type conversion based on type
            let conversion = type_conversion(&opt.typ, name, &opt.default);
            method.line(format!("        {}", conversion));
        }

        method.line("    },");
    }

    method.line("};");
    method.line("");
    method.line("let sources = SettingsSources {");
    method.line("    sources: self.sources,");
    method.line("};");
    method.line("");
    method.line("(settings, sources)");

    Ok(())
}

fn type_conversion(typ: &str, name: &str, default: &toml::Value) -> String {
    match typ {
        "bool" => {
            let default_val = match default {
                toml::Value::Boolean(b) => b.to_string(),
                _ => "false".to_string(),
            };
            format!("val.and_then(|v| v.as_bool()).unwrap_or({})", default_val)
        }
        "int" => {
            if name == "jobs" {
                "val.and_then(|v| v.as_integer()).and_then(|i| NonZero::new(i as usize)).unwrap_or_else(|| NonZero::new(4).unwrap())".to_string()
            } else if name == "verbose" {
                "val.and_then(|v| v.as_integer()).and_then(|i| NonZero::new(i as usize)).unwrap_or_else(|| NonZero::new(1).unwrap())".to_string()
            } else {
                "val.and_then(|v| v.as_integer()).unwrap_or(0) as usize".to_string()
            }
        }
        "string" | "enum" => {
            let default_str = match default {
                toml::Value::String(s) => format!("\"{}\"", s),
                _ => "\"\"".to_string(),
            };
            format!("val.and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_else(|| {}.to_string())", default_str)
        }
        "path" => {
            let default_path = match default {
                toml::Value::String(s) if !s.is_empty() => format!("\"{}\"", s),
                _ => "\"\"".to_string(),
            };
            format!("val.and_then(|v| v.as_str().map(|s| PathBuf::from(s))).unwrap_or_else(|| PathBuf::from({}))", default_path)
        }
        typ if typ.starts_with("list<") => {
            "val.and_then(|v| v.as_array().cloned()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or_else(IndexSet::new)".to_string()
        }
        _ => "Default::default()".to_string()
    }
}

fn generate_cli_flags(
    registry: &SettingsRegistry,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();

    scope.raw("#[allow(dead_code)]");
    scope.import("clap", "Args");
    scope.import("std::num", "NonZero");
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

fn generate_git_keys(
    registry: &SettingsRegistry,
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut scope = Scope::new();

    scope.raw("#[allow(dead_code)]");

    let git_keys_struct = scope.new_struct("GitConfigKeys").vis("pub");

    for (name, opt) in &registry.option {
        if opt.sources.git.is_empty() {
            continue;
        }

        let field_name = name.replace('-', "_");
        git_keys_struct.field(&format!("pub {}", field_name), "Vec<&'static str>");
    }

    // Generate a function to initialize the struct
    let impl_block = scope.new_impl("GitConfigKeys");
    let new_fn = impl_block.new_fn("new").vis("pub").ret("Self");

    new_fn.line("GitConfigKeys {");
    for (name, opt) in &registry.option {
        if opt.sources.git.is_empty() {
            continue;
        }

        let field_name = name.replace('-', "_");
        let keys = opt
            .sources
            .git
            .iter()
            .map(|k| format!("\"{}\"", k))
            .collect::<Vec<_>>()
            .join(", ");
        new_fn.line(format!("    {}: vec![{}],", field_name, keys));
    }
    new_fn.line("}");

    // Use once_cell for the static
    let mut const_init = String::from("\nuse once_cell::sync::Lazy;\n");
    const_init.push_str(
        "pub static GIT_CONFIG_KEYS: Lazy<GitConfigKeys> = Lazy::new(GitConfigKeys::new);",
    );

    scope.raw(&const_init);

    fs::write(out_dir.join("generated_git_keys.rs"), scope.to_string())?;
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
