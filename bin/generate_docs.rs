use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

// Include the shared types from the build directory
#[path = "../build/settings_toml.rs"]
mod settings_toml;

use settings_toml::{PklSource, SettingsRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("docs/gen")?;
    generate_settings_doc()?;
    println!("Generated docs/gen/settings-config.md");
    generate_builtins_doc()?;
    println!("Generated docs/gen/builtins.md");
    generate_pkl_config_doc()?;
    println!("Generated docs/gen/pkl-config.md");

    Ok(())
}

fn generate_settings_doc() -> Result<(), Box<dyn std::error::Error>> {
    let settings_content = fs::read_to_string("settings.toml")?;
    let registry: SettingsRegistry = toml::from_str(&settings_content)?;

    let mut md = String::new();

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

    md = md.trim().to_string();
    md.push('\n');

    fs::write("docs/gen/settings-config.md", md)?;
    Ok(())
}

/// A builtin step with its metadata from annotations
struct BuiltinInfo {
    name: String,
    category: String,
    description: String,
    step: serde_json::Value,
}

fn generate_builtins_doc() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    let reflect_script = cwd.join("scripts/reflect.pkl");
    let reflect_uri = format!("file://{}", reflect_script.display());

    // Get list of builtin files
    let builtins_dir = cwd.join("pkl/builtins");
    let mut builtin_files: Vec<_> = fs::read_dir(&builtins_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "pkl"))
        .collect();
    builtin_files.sort_by_key(|e| e.path());

    let mut builtins_info: Vec<BuiltinInfo> = Vec::new();

    for entry in builtin_files {
        let path = entry.path();

        // Use the reflector to get metadata from annotations
        let reflect_expr = format!(r#"import("{}").render(module)"#, reflect_uri);
        let output = Command::new("pkl")
            .args([
                "eval",
                &path.to_string_lossy(),
                "--format",
                "json",
                "-x",
                &reflect_expr,
            ])
            .output()?;

        if !output.status.success() {
            eprintln!(
                "Warning: Failed to reflect {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            );
            continue;
        }

        let json_str = String::from_utf8(output.stdout)?;
        let reflected: serde_json::Value = serde_json::from_str(&json_str)?;

        // Extract metadata from the reflected module
        let module_class = reflected.get("moduleClass");
        let properties = module_class.and_then(|mc| mc.get("properties"));

        if let Some(serde_json::Value::Object(props)) = properties {
            for (name, prop) in props {
                // Get annotations from the property
                let annotations = prop.get("annnotations"); // Note: typo in reflect.pkl
                let (category, description) =
                    if let Some(serde_json::Value::Array(anns)) = annotations {
                        let mut cat = "Uncategorized".to_string();
                        let mut desc = String::new();
                        for ann in anns {
                            if let Some(c) = ann.get("category").and_then(|v| v.as_str()) {
                                cat = c.to_string();
                            }
                            if let Some(d) = ann.get("description").and_then(|v| v.as_str()) {
                                desc = d.to_string();
                            }
                        }
                        (cat, desc)
                    } else {
                        ("Uncategorized".to_string(), String::new())
                    };

                let step: serde_json::Value =
                    serde_json::from_str(prop.get("defaultValue").unwrap().as_str().unwrap())?;

                builtins_info.push(BuiltinInfo {
                    name: name.clone(),
                    category,
                    description,
                    step,
                });
            }
        }
    }

    // Group by category
    let mut by_category: BTreeMap<String, Vec<&BuiltinInfo>> = BTreeMap::new();
    for info in &builtins_info {
        by_category
            .entry(info.category.clone())
            .or_default()
            .push(info);
    }

    // Sort builtins within each category
    for builtins in by_category.values_mut() {
        builtins.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let mut md = String::new();

    // Generate markdown grouped by category
    for (category, builtins) in &by_category {
        md.push_str(&format!("## {}\n\n", category));

        for info in builtins {
            // Format the builtin name with underscores replaced by hyphens for display
            let display_name = info.name.replace('_', "-");
            md.push_str(&format!("### `{}`\n\n", display_name));

            if !info.description.is_empty() {
                md.push_str(&format!("{}\n\n", info.description));
            }

            // Show glob pattern(s)
            if let Some(glob) = info.step.get("glob") {
                let glob_str = match glob {
                    serde_json::Value::String(s) => format!("`{}`", s),
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| format!("`{}`", s))
                        .collect::<Vec<_>>()
                        .join(", "),
                    _ => String::new(),
                };
                if !glob_str.is_empty() {
                    md.push_str(&format!("- **Glob:** {}\n", glob_str));
                }
            }

            // Show check command if present (check, check_diff, or check_list_files)
            if let Some(check) = info.step.get("check").and_then(|v| v.as_str()) {
                if !check.is_empty() {
                    md.push_str(&format!("- **Check:** `{}`\n", check));
                }
            } else if let Some(check_diff) = info.step.get("check_diff").and_then(|v| v.as_str()) {
                if !check_diff.is_empty() {
                    md.push_str(&format!("- **Check (diff):** `{}`\n", check_diff));
                }
            } else if let Some(check_list) =
                info.step.get("check_list_files").and_then(|v| v.as_str())
                && !check_list.is_empty()
            {
                md.push_str(&format!("- **Check (list-files):** `{}`\n", check_list));
            }

            // Show fix command if present
            if let Some(fix) = info.step.get("fix").and_then(|v| v.as_str())
                && !fix.is_empty()
            {
                md.push_str(&format!("- **Fix:** `{}`\n", fix));
            }

            md.push('\n');
        }
    }

    md = md.trim().to_string();
    md.push('\n');

    fs::write("docs/gen/builtins.md", md)?;
    Ok(())
}

fn format_property_doc(name: &str, value: &serde_json::Value, heading_level: &str) -> String {
    let mut doc = format!(
        "{} `{}: {}`\n\n",
        heading_level,
        name,
        value["type"]
            .as_str()
            .unwrap()
            .trim_end_matches("?")
            .replace("RegexPattern", "Regex")
    );
    if let Some(doc_comment) = value["docComment"].as_str() {
        doc.push_str(doc_comment);
        doc.push_str("\n\n");
    }
    doc
}

fn generate_pkl_config_doc() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    let cwd = std::env::current_dir()?;
    let reflect_script = cwd.join("scripts/reflect.pkl");
    let reflect_uri = format!("file://{}", reflect_script.display());
    let output = Command::new("pkl")
        .arg("eval")
        .arg("pkl/Config.pkl")
        .arg("--format")
        .arg("json")
        .arg("-x")
        .arg(format!("import(\"{}\").render(module)", reflect_uri))
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "pkl eval failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let config_json_str = String::from_utf8(output.stdout)?;
    let config_json_str = config_json_str.replace("https://hk.jdx.dev/", "/");
    let config_json: serde_json::Value = serde_json::from_str(&config_json_str)?;

    let mut md = String::new();

    // Process top-level properties from moduleClass
    let properties = config_json["moduleClass"]["properties"]
        .as_object()
        .expect("Expected top level properties in Config.pkl");
    for (key, value) in properties {
        match key.as_str() {
            "output" => continue,
            "min_hk_version" => continue,
            // TODO(thejcannon): Include these
            "display_skip_reasons" => continue,
            "warnings" => continue,
            "stage" => continue,
            _ => (),
        }
        md.push_str(&format_property_doc(key, value, "##"));
    }

    // Process hooks
    md.push_str("## `hooks.<HOOK>`\n\n");
    let properties = config_json["classes"]["Hook"]["properties"]
        .as_object()
        .expect("Expected Hook class in Config.pkl!");
    for (key, value) in properties {
        if key == "_type" {
            continue;
        }
        md.push_str(&format_property_doc(
            &format!("<HOOK>.{}", key),
            value,
            "###",
        ));
    }

    // Process Steps
    md.push_str("## `hooks.<HOOK>.steps.<STEP|GROUP>`\n\n");
    let properties = config_json["classes"]["Step"]["properties"]
        .as_object()
        .expect("Expected Step class in Config.pkl");
    for (key, value) in properties {
        if key == "_type" {
            continue;
        }
        md.push_str(&format_property_doc(
            &format!("<STEP>.{}", key),
            value,
            "###",
        ));
    }

    md = md.trim().to_string();
    md.push('\n');

    fs::write("docs/gen/pkl-config.md", md)?;

    Ok(())
}
