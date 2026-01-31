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
}
