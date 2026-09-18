use std::path::{Path, PathBuf};

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

    let source_files = collect_source_files(project_root);
    for meta in BUILTINS_META {
        // Skip builtins without project indicators
        if meta.project_indicators.is_empty() {
            continue;
        }

        // Check if any indicator matches
        for indicator in meta.project_indicators {
            if let Some(reason) = matches_indicator(project_root, indicator, &source_files) {
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

/// Collect real source files recursively, honoring repository ignore rules.
fn collect_source_files(project_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
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
        match entry {
            Ok(entry) if entry.file_type().is_some_and(|kind| kind.is_file()) => {
                files.push(entry.into_path())
            }
            Ok(_) => {}
            Err(error) => warn!("Unable to inspect project file: {error}"),
        }
    }
    files
}

/// Check if a project indicator matches and return the reason if it does.
fn matches_indicator(
    project_root: &Path,
    indicator: &crate::builtins::ProjectIndicator,
    source_files: &[PathBuf],
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
    let pattern = if pattern.starts_with("*.") {
        format!("**/{pattern}")
    } else {
        pattern.to_string()
    };
    let matcher = match globset::Glob::new(&pattern) {
        Ok(glob) => glob.compile_matcher(),
        Err(error) => {
            warn!("Invalid project indicator glob {pattern:?}: {error}");
            return None;
        }
    };
    if source_files.iter().any(|path| {
        path.strip_prefix(project_root)
            .ok()
            .is_some_and(|relative| matcher.is_match(relative))
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
