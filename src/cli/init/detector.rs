use std::path::Path;

use crate::builtins::{BUILTINS_META, BuiltinMeta};

/// Detection result for project files
#[derive(Debug)]
pub struct Detection {
    pub builtin: &'static BuiltinMeta,
    pub reason: String,
}

/// Detect relevant builtins for the current project based on project_indicators
pub fn detect_builtins(project_root: &Path) -> Vec<Detection> {
    let mut detections = Vec::new();

    for meta in BUILTINS_META {
        // Skip builtins without project indicators
        if meta.project_indicators.is_empty() {
            continue;
        }

        // Check if any indicator matches
        for indicator in meta.project_indicators {
            if let Some(reason) = matches_indicator(project_root, indicator) {
                detections.push(Detection {
                    builtin: meta,
                    reason,
                });
                break; // Only add each builtin once
            }
        }
    }

    detections
}

/// Check if a project indicator matches and return the reason if it does
fn matches_indicator(
    project_root: &Path,
    indicator: &crate::builtins::ProjectIndicator,
) -> Option<String> {
    // Handle file indicator (exact file or directory match)
    if let Some(file) = indicator.file {
        let path = project_root.join(file);
        if !path.exists() {
            return None;
        }

        // If contains is specified, grep the file
        if let Some(pattern) = indicator.contains {
            if path.is_file()
                && let Ok(content) = std::fs::read_to_string(&path)
                && content.contains(pattern)
            {
                return Some(format!("{} contains {}", file, pattern));
            }
            return None;
        }

        return Some(file.to_string());
    }

    // Handle glob indicator
    if let Some(glob_pattern) = indicator.glob
        && let Some(ext) = glob_pattern.strip_prefix("*.")
        && let Ok(entries) = std::fs::read_dir(project_root)
    {
        for entry in entries.flatten() {
            if let Some(file_ext) = entry.path().extension()
                && file_ext == ext
            {
                return Some(format!("{} files", glob_pattern));
            }
        }
    }

    None
}

/// Select a curated JavaScript/TypeScript toolset for a setup preset.
/// Native configuration and declared dependencies take precedence over presets.
const FAST_JS_TOOLS: &[&str] = &["biome"];
const ECOSYSTEM_JS_TOOLS: &[&str] = &["eslint", "prettier"];

pub(crate) fn preset_tool_names(project_root: &Path, preset: &str) -> Vec<&'static str> {
    let package = project_root.join("package.json");
    let package_text = std::fs::read_to_string(&package).unwrap_or_default();
    let package_json: serde_json::Value = serde_json::from_str(&package_text).unwrap_or_default();
    let dep = |name: &str| {
        [
            "dependencies",
            "devDependencies",
            "peerDependencies",
            "optionalDependencies",
        ]
        .iter()
        .any(|section| {
            package_json
                .get(section)
                .and_then(|v| v.get(name))
                .is_some()
        })
    };
    let has_js = package.exists()
        || std::fs::read_dir(project_root)
            .map(|entries| {
                entries.flatten().any(|entry| {
                    entry.path().is_file()
                        && matches!(
                            entry.path().extension().and_then(|e| e.to_str()),
                            Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "mts" | "cjs" | "cts")
                        )
                })
            })
            .unwrap_or(false);
    let has_biome = project_root.join("biome.json").exists()
        || project_root.join("biome.jsonc").exists()
        || dep("@biomejs/biome");
    let has_eslint = dep("eslint")
        || package_json.get("eslintConfig").is_some()
        || [
            "eslint.config.js",
            "eslint.config.mjs",
            "eslint.config.cjs",
            "eslint.config.ts",
            "eslint.config.cts",
            "eslint.config.mts",
            ".eslintrc",
            ".eslintrc.json",
            ".eslintrc.js",
            ".eslintrc.cjs",
            ".eslintrc.yaml",
            ".eslintrc.yml",
        ]
        .iter()
        .any(|n| project_root.join(n).exists());
    let has_prettier = dep("prettier")
        || package_json.get("prettier").is_some()
        || [
            ".prettierrc",
            ".prettierrc.json",
            ".prettierrc.js",
            ".prettierrc.cjs",
            ".prettierrc.mjs",
            ".prettierrc.ts",
            ".prettierrc.mts",
            ".prettierrc.cts",
            ".prettierrc.json5",
            ".prettierrc.yaml",
            ".prettierrc.yml",
            ".prettierrc.toml",
            "prettier.config.js",
            "prettier.config.cjs",
            "prettier.config.mjs",
            "prettier.config.ts",
            "prettier.config.cts",
            "prettier.config.mts",
        ]
        .iter()
        .any(|n| project_root.join(n).exists());
    let has_other_formatter = dep("oxfmt")
        || dep("deno")
        || project_root.join("deno.json").exists()
        || project_root.join("deno.jsonc").exists()
        || project_root.join(".oxfmtrc.json").exists()
        || project_root.join(".oxfmtrc.jsonc").exists()
        || project_root.join("oxfmt.config.ts").exists();
    if !has_js && !has_biome && !has_eslint && !has_prettier && !has_other_formatter {
        return Vec::new();
    }
    let mut names = Vec::new();
    if has_biome {
        names.push("biome");
    }
    if has_eslint {
        names.push("eslint");
    }
    if has_prettier {
        names.push("prettier");
    }
    if names.is_empty() && !has_other_formatter {
        let defaults: &[&str] = match preset {
            "fast" => FAST_JS_TOOLS,
            "ecosystem" => ECOSYSTEM_JS_TOOLS,
            _ => &[],
        };
        names.extend(defaults.iter().copied());
    } else if !has_biome && !has_prettier && !has_other_formatter && preset == "ecosystem" {
        names.push("prettier");
    }
    names
}

pub fn detect_preset_builtins(project_root: &Path, preset: &str) -> Vec<&'static BuiltinMeta> {
    preset_tool_names(project_root, preset)
        .into_iter()
        .filter_map(|name| BUILTINS_META.iter().find(|meta| meta.name == name))
        .collect()
}

pub fn preset_builtins_with_detections(
    project_root: &Path,
    detections: &[Detection],
    preset: &str,
) -> Vec<&'static BuiltinMeta> {
    let mut selected = detect_preset_builtins(project_root, preset);
    for detection in detections {
        if matches!(detection.builtin.name, "biome" | "eslint" | "prettier") {
            continue;
        }
        if !selected
            .iter()
            .any(|meta| meta.name == detection.builtin.name)
        {
            selected.push(detection.builtin);
        }
    }
    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_builtins_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let detections = detect_builtins(tmp.path());
        assert!(detections.is_empty());
    }

    #[test]
    fn test_detect_builtins_with_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        let detections = detect_builtins(tmp.path());

        let names: Vec<_> = detections.iter().map(|d| d.builtin.name).collect();
        assert!(names.contains(&"cargo_clippy"));
        assert!(names.contains(&"cargo_fmt"));
    }

    #[test]
    fn test_detect_builtins_with_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let detections = detect_builtins(tmp.path());

        let names: Vec<_> = detections.iter().map(|d| d.builtin.name).collect();
        assert!(names.contains(&"prettier"));
    }

    #[test]
    fn test_detect_eslint_with_contains() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"devDependencies": {"eslint": "^8.0.0"}}"#,
        )
        .unwrap();
        let detections = detect_builtins(tmp.path());

        let names: Vec<_> = detections.iter().map(|d| d.builtin.name).collect();
        assert!(names.contains(&"eslint"));
    }

    #[test]
    fn test_detect_eslint_not_present() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"devDependencies": {"prettier": "^3.0.0"}}"#,
        )
        .unwrap();
        let detections = detect_builtins(tmp.path());

        let names: Vec<_> = detections.iter().map(|d| d.builtin.name).collect();
        assert!(!names.contains(&"eslint"));
    }

    #[test]
    fn test_detect_shell_scripts() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("test.sh"), "#!/bin/bash\necho hello").unwrap();
        let detections = detect_builtins(tmp.path());

        let names: Vec<_> = detections.iter().map(|d| d.builtin.name).collect();
        assert!(names.contains(&"shellcheck"));
    }

    #[test]
    fn preset_fast_prefers_biome_without_existing_tools() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        assert_eq!(preset_tool_names(tmp.path(), "fast"), vec!["biome"]);
    }

    #[test]
    fn preset_ecosystem_recommends_eslint_and_prettier() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        assert_eq!(
            preset_tool_names(tmp.path(), "ecosystem"),
            vec!["eslint", "prettier"]
        );
    }

    #[test]
    fn preset_fast_does_not_readd_weak_prettier_detection() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let detections = detect_builtins(tmp.path());
        let selected = preset_builtins_with_detections(tmp.path(), &detections, "fast");
        let names: Vec<_> = selected.iter().map(|meta| meta.name).collect();
        assert_eq!(names, vec!["biome"]);
    }

    #[test]
    fn preset_does_not_add_tools_when_other_formatter_is_configured() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"devDependencies":{"oxfmt":"latest"}}"#,
        )
        .unwrap();
        assert!(preset_tool_names(tmp.path(), "fast").is_empty());
    }

    #[test]
    fn preset_does_not_add_prettier_when_biome_is_configured() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"devDependencies":{"@biomejs/biome":"latest"}}"#,
        )
        .unwrap();
        assert_eq!(preset_tool_names(tmp.path(), "ecosystem"), vec!["biome"]);
    }
    #[test]
    fn preset_preserves_biome_config_without_source_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("biome.json"), "{}").unwrap();
        let detections = detect_builtins(tmp.path());
        let selected = preset_builtins_with_detections(tmp.path(), &detections, "fast");
        assert!(selected.iter().any(|meta| meta.name == "biome"));
    }

    #[test]
    fn preset_preserves_oxfmt_config_and_does_not_add_biome() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("oxfmt.config.ts"), "export default {}").unwrap();
        let detections = detect_builtins(tmp.path());
        let selected = preset_builtins_with_detections(tmp.path(), &detections, "fast");
        let names: Vec<_> = selected.iter().map(|meta| meta.name).collect();
        assert!(names.contains(&"oxfmt"));
        assert!(!names.contains(&"biome"));
    }

    #[test]
    fn preset_fast_detects_root_cjs_file_but_not_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("index.cjs"), "").unwrap();
        assert_eq!(preset_tool_names(tmp.path(), "fast"), vec!["biome"]);
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("fake.cjs")).unwrap();
        assert!(preset_tool_names(dir.path(), "fast").is_empty());
    }
}
