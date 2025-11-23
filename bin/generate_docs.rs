use std::fs;

// Include the shared types from the build directory
#[path = "../build/settings_toml.rs"]
mod settings_toml;

use settings_toml::{PklSource, SettingsRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings_content = fs::read_to_string("settings.toml")?;
    let registry: SettingsRegistry = toml::from_str(&settings_content)?;

    generate_configuration_docs(&registry)?;
    println!("Generated docs/gen/settings-config.md");

    Ok(())
}

fn generate_configuration_docs(
    registry: &SettingsRegistry,
) -> Result<(), Box<dyn std::error::Error>> {
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
