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

fn generate_builtins_doc() -> Result<(), Box<dyn std::error::Error>> {
    // Run pkl eval to get JSON output
    let output = Command::new("pkl")
        .args(["eval", "pkl/Builtins.pkl", "--format", "json"])
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "pkl eval failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let json_str = String::from_utf8(output.stdout)?;
    let builtins: serde_json::Value = serde_json::from_str(&json_str)?;

    // Collect builtins grouped by category
    let mut by_category: BTreeMap<String, Vec<(String, &serde_json::Value)>> = BTreeMap::new();

    if let serde_json::Value::Object(map) = &builtins {
        for (name, value) in map {
            let category = value
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("Uncategorized")
                .to_string();
            by_category
                .entry(category)
                .or_default()
                .push((name.clone(), value));
        }
    }

    // Sort builtins within each category
    for builtins in by_category.values_mut() {
        builtins.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let mut md = String::new();

    // Generate markdown grouped by category
    for (category, builtins) in &by_category {
        md.push_str(&format!("## {}\n\n", category));

        for (name, value) in builtins {
            let description = value
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Format the builtin name with underscores replaced by hyphens for display
            let display_name = name.replace('_', "-");
            md.push_str(&format!("### `{}`\n\n", display_name));

            if !description.is_empty() {
                md.push_str(&format!("{}\n\n", description));
            }

            // Show glob pattern(s)
            if let Some(glob) = value.get("glob") {
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
            if let Some(check) = value.get("check").and_then(|v| v.as_str()) {
                if !check.is_empty() {
                    md.push_str(&format!("- **Check:** `{}`\n", check));
                }
            } else if let Some(check_diff) = value.get("check_diff").and_then(|v| v.as_str()) {
                if !check_diff.is_empty() {
                    md.push_str(&format!("- **Check (diff):** `{}`\n", check_diff));
                }
            } else if let Some(check_list) = value.get("check_list_files").and_then(|v| v.as_str())
            {
                if !check_list.is_empty() {
                    md.push_str(&format!("- **Check (list-files):** `{}`\n", check_list));
                }
            }

            // Show fix command if present
            if let Some(fix) = value.get("fix").and_then(|v| v.as_str()) {
                if !fix.is_empty() {
                    md.push_str(&format!("- **Fix:** `{}`\n", fix));
                }
            }

            md.push('\n');
        }
    }

    md = md.trim().to_string();
    md.push('\n');

    fs::write("docs/gen/builtins.md", md)?;
    Ok(())
}
