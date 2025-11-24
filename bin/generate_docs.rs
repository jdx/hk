use std::fs;

// Include the shared types from the build directory
#[path = "../build/settings_toml.rs"]
mod settings_toml;

use settings_toml::{PklSource, SettingsRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_settings_config_doc()?;
    println!("Generated docs/gen/settings-config.md");
    generate_pkl_config_doc()?;
    println!("Generated docs/gen/pkl-config.md");

    Ok(())
}

fn generate_settings_config_doc() -> Result<(), Box<dyn std::error::Error>> {
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

fn generate_pkl_config_doc() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    let reflector_path = std::env::current_dir()?.join("pkl/_reflector.pkl");
    let reflector_uri = format!("file:{}", reflector_path.display());

    let output = Command::new("pkl")
        .arg("eval")
        .arg("pkl/Config.pkl")
        .arg("--format")
        .arg("json")
        .arg("-x")
        .arg(format!("import(\"{}\").render(module)", reflector_uri))
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "pkl command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let config_json_str = String::from_utf8(output.stdout)?;
    let config_json_str = config_json_str.replace("https://hk.jdx.dev/", "/");
    let config_json: serde_json::Value = serde_json::from_str(&config_json_str)?;

    let mut md = String::new();

    // Process top-level properties from moduleClass
    if let Some(properties) = config_json["moduleClass"]["properties"].as_object() {
        for (key, value) in properties {
            // Skip internal properties like "output"
            if key == "output" {
                continue;
            }

            // TODO(josh_cannon): Make all types nullable
            md.push_str(&format!(
                "## `{}: {}`\n\n",
                key,
                value["type"].as_str().unwrap_or("Unknown")
            ));
            if let Some(default) = value["defaultValue"].as_str() {
                if default != "null" {
                    md.push_str(&format!("Default: `{}`\n\n", default));
                }
            }
            if let Some(doc) = value["docComment"].as_str() {
                md.push_str(doc);
                md.push_str("\n\n");
            }
        }
    }

    // Process hooks
    md.push_str("## `hooks.<HOOK>`\n\n");
    if let Some(properties) = config_json["classes"]["Hook"]["properties"].as_object() {
        for (key, value) in properties {
            // Skip internal properties like "output"
            if key == "output" {
                continue;
            }

            // TODO(josh_cannon): Make all types nullable
            md.push_str(&format!(
                "### `<HOOK>.{}: {}`\n\n",
                key,
                value["type"].as_str().unwrap_or("Unknown")
            ));
            if let Some(default) = value["defaultValue"].as_str() {
                if default != "null" {
                    md.push_str(&format!("Default: `{}`\n\n", default));
                }
            }
            if let Some(doc) = value["docComment"].as_str() {
                md.push_str(doc);
                md.push_str("\n\n");
            }
        }
    }

    // Process Steps
    md.push_str("## `hooks.<HOOK>.steps.<STEP|GROUP>`\n\n");
    if let Some(properties) = config_json["classes"]["Step"]["properties"].as_object() {
        for (key, value) in properties {
            // Skip internal properties like "output"
            if key == "output" {
                continue;
            }

            // TODO(josh_cannon): Make all types nullable
            md.push_str(&format!(
                "### `<STEP>.{}: {}`\n\n",
                key,
                value["type"].as_str().unwrap_or("Unknown")
            ));
            if let Some(default) = value["defaultValue"].as_str() {
                if default != "null" {
                    md.push_str(&format!("Default: `{}`\n\n", default));
                }
            }
            if let Some(doc) = value["docComment"].as_str() {
                md.push_str(doc);
                md.push_str("\n\n");
            }
        }
    }

    md = md.trim().to_string();
    md.push('\n');

    fs::write("docs/gen/pkl-config.md", md)?;
    Ok(())
}
