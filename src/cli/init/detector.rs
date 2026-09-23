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
    let mut recursive_indicators = Vec::new();
    let mut root_matches = Vec::new();

    for meta in BUILTINS_META {
        // Skip builtins without project indicators
        if meta.project_indicators.is_empty() {
            continue;
        }

        // Check if any indicator matches
        for (indicator_index, indicator) in meta.project_indicators.iter().enumerate() {
            let recursive = indicator.recursive
                || indicator
                    .glob
                    .is_some_and(|pattern| pattern.starts_with("**/"));
            if recursive {
                if let Some(pattern) = indicator.glob {
                    match compile_indicator(pattern, true) {
                        Some(matcher) => {
                            recursive_indicators.push((meta, indicator_index, indicator, matcher))
                        }
                        None => continue,
                    }
                }
                continue;
            }
            if let Some(reason) = matches_root_indicator(project_root, indicator) {
                root_matches.push((meta, indicator_index, reason));
            }
        }
    }

    // Match recursive source indicators as files are visited. Do not retain the
    // whole tree: once an indicator is satisfied it no longer needs matching.
    let mut satisfied = vec![false; recursive_indicators.len()];
    let walker = ignore::WalkBuilder::new(project_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .parents(true)
        .ignore(true)
        .follow_links(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();
    for entry in walker {
        let Ok(entry) = entry else {
            if let Err(error) = entry {
                warn!("Unable to inspect project file: {error}");
            }
            continue;
        };
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Some(relative) = entry.path().strip_prefix(project_root).ok() else {
            continue;
        };
        for (index, (_, _, _, matcher)) in recursive_indicators.iter().enumerate() {
            if !satisfied[index] && matcher.is_match(relative) {
                satisfied[index] = true;
            }
        }
        if satisfied.iter().all(|matched| *matched) {
            break;
        }
    }
    // Select the first matching indicator for each builtin, preserving the
    // declaration order when a builtin has multiple indicators.
    let mut detections = Vec::new();
    for meta in BUILTINS_META {
        for (indicator_index, indicator) in meta.project_indicators.iter().enumerate() {
            if let Some((_, _, reason)) = root_matches.iter().find(|(candidate, index, _)| {
                std::ptr::eq(*candidate, meta) && *index == indicator_index
            }) {
                detections.push(Detection {
                    builtin: meta,
                    reason: reason.clone(),
                });
                break;
            }
            if recursive_indicators.iter().enumerate().any(
                |(recursive_index, (candidate, index, _, _))| {
                    std::ptr::eq(*candidate, meta)
                        && *index == indicator_index
                        && satisfied[recursive_index]
                },
            ) {
                detections.push(Detection {
                    builtin: meta,
                    reason: format!("{} files", indicator.glob.unwrap()),
                });
                break;
            }
        }
    }

    detections
}

fn compile_indicator(pattern: &str, recursive: bool) -> Option<globset::GlobMatcher> {
    let pattern = if recursive && pattern.starts_with("*.") {
        format!("**/{pattern}")
    } else {
        pattern.to_string()
    };
    match globset::Glob::new(&pattern) {
        Ok(glob) => Some(glob.compile_matcher()),
        Err(error) => {
            warn!("Invalid project indicator glob {pattern:?}: {error}");
            None
        }
    }
}

/// Check a root-only project indicator.
fn matches_root_indicator(
    project_root: &Path,
    indicator: &crate::builtins::ProjectIndicator,
) -> Option<String> {
    if let Some(file) = indicator.file {
        let path = project_root.join(file);
        if !path.exists() {
            return None;
        }
        if let Some(pattern) = indicator.contains {
            if path.is_file()
                && std::fs::read_to_string(&path).is_ok_and(|content| content.contains(pattern))
            {
                return Some(format!("{} contains {}", file, pattern));
            }
            return None;
        }
        return Some(file.to_string());
    }
    let pattern = indicator.glob?;
    let matcher = compile_indicator(pattern, false)?;
    if std::fs::read_dir(project_root).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_file())
                && matcher.is_match(entry.file_name())
        })
    }) {
        Some(format!("{} files", indicator.glob.unwrap()))
    } else {
        None
    }
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
    fn test_detect_biome_jsonc() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("biome.jsonc"), "{}").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|detection| detection.builtin.name)
            .collect();
        assert!(names.contains(&"biome"));
    }

    #[test]
    fn test_detect_eslint_config_files() {
        for filename in [
            "eslint.config.js",
            "eslint.config.mjs",
            "eslint.config.cjs",
            "eslint.config.ts",
            "eslint.config.mts",
            "eslint.config.cts",
            ".eslintrc",
            ".eslintrc.json",
            ".eslintrc.yaml",
            ".eslintrc.yml",
            ".eslintrc.js",
            ".eslintrc.cjs",
        ] {
            let tmp = tempfile::tempdir().unwrap();
            std::fs::write(tmp.path().join(filename), "{}").unwrap();
            let names: Vec<_> = detect_builtins(tmp.path())
                .iter()
                .map(|detection| detection.builtin.name)
                .collect();
            assert!(
                names.contains(&"eslint"),
                "expected detection for {filename}"
            );
        }
    }

    #[test]
    fn test_detect_git_only_scripts() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/objects")).unwrap();
        std::fs::write(tmp.path().join(".git/objects/internal.sh"), "echo no").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(!names.contains(&"shellcheck"));
    }

    #[test]
    fn test_detect_ignore_file_outside_git() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".ignore"), "ignored/\n").unwrap();
        std::fs::create_dir(tmp.path().join("ignored")).unwrap();
        std::fs::write(tmp.path().join("ignored/skip.sh"), "echo no").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(!names.contains(&"shellcheck"));
    }

    #[test]
    fn test_nested_manifest_does_not_activate_root_indicator() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("nested")).unwrap();
        std::fs::write(tmp.path().join("nested/Cargo.toml"), "[package]").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(!names.contains(&"cargo_clippy"));
    }

    #[test]
    fn test_detect_nested_shell_script_but_not_ignored_or_git_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src/bin")).unwrap();
        std::fs::write(tmp.path().join("src/bin/check.sh"), "echo ok").unwrap();
        std::fs::write(tmp.path().join("src/bin/check.bash"), "echo ok").unwrap();
        std::fs::create_dir_all(tmp.path().join("ignored")).unwrap();
        std::fs::write(tmp.path().join("ignored/.gitignore"), "*.sh\n").unwrap();
        std::fs::write(tmp.path().join("ignored/skip.sh"), "echo no").unwrap();
        std::fs::create_dir_all(tmp.path().join(".git/objects")).unwrap();
        std::fs::write(tmp.path().join(".git/objects/internal.sh"), "echo no").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(names.contains(&"shellcheck"));
        assert!(names.contains(&"shellharden"));
        assert_eq!(
            names.iter().filter(|name| **name == "shellharden").count(),
            1
        );
        let shellharden = detect_builtins(tmp.path())
            .into_iter()
            .find(|detection| detection.builtin.name == "shellharden")
            .unwrap();
        assert_eq!(shellharden.reason, "*.sh files");
    }

    #[test]
    fn test_detect_ignored_only_scripts() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "ignored/\n").unwrap();
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(tmp.path())
            .status()
            .unwrap();
        std::fs::create_dir(tmp.path().join("ignored")).unwrap();
        std::fs::write(tmp.path().join("ignored/skip.sh"), "echo no").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(!names.contains(&"shellcheck"));
    }

    #[test]
    fn test_detect_hidden_source_and_not_source_named_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".hidden-check.sh"), "echo ok").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(names.contains(&"shellcheck"));
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("fake.sh")).unwrap();
        let names: Vec<_> = detect_builtins(dir.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(!names.contains(&"shellcheck"));
    }

    #[test]
    fn test_detect_nested_build_bazel() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("nested")).unwrap();
        std::fs::write(tmp.path().join("nested/BUILD.bazel"), "").unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(names.contains(&"buildifier_lint"));
        assert!(names.contains(&"buildifier_format"));
    }

    #[test]
    fn test_dotnet_manifests_are_root_only_for_all_extensions() {
        for extension in ["csproj", "vbproj", "sln", "slnx"] {
            let root = tempfile::tempdir().unwrap();
            std::fs::write(root.path().join(format!("project.{extension}")), "").unwrap();
            let names: Vec<_> = detect_builtins(root.path())
                .iter()
                .map(|d| d.builtin.name)
                .collect();
            assert!(names.contains(&"dotnet_format"), "root .{extension}");

            let nested = tempfile::tempdir().unwrap();
            std::fs::create_dir(nested.path().join("nested")).unwrap();
            std::fs::write(
                nested.path().join(format!("nested/project.{extension}")),
                "",
            )
            .unwrap();
            let names: Vec<_> = detect_builtins(nested.path())
                .iter()
                .map(|d| d.builtin.name)
                .collect();
            assert!(!names.contains(&"dotnet_format"), "nested .{extension}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_detect_does_not_follow_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("outside.sh"), "echo no").unwrap();
        std::os::unix::fs::symlink(outside.path(), tmp.path().join("linked")).unwrap();
        let names: Vec<_> = detect_builtins(tmp.path())
            .iter()
            .map(|d| d.builtin.name)
            .collect();
        assert!(!names.contains(&"shellcheck"));
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
