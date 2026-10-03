use crate::Result;
use crate::step::Pattern;
use globset::{GlobBuilder, GlobSetBuilder};
use itertools::Itertools;
use regex::Regex;
use std::path::{Path, PathBuf};

pub fn get_matches<P: AsRef<Path>>(glob: &[String], files: &[P]) -> Result<Vec<PathBuf>> {
    get_matches_with_options(glob, files, false)
}

pub fn get_matches_strict<P: AsRef<Path>>(glob: &[String], files: &[P]) -> Result<Vec<PathBuf>> {
    get_matches_with_options(glob, files, true)
}

fn get_matches_with_options<P: AsRef<Path>>(
    glob: &[String],
    files: &[P],
    literal_separator: bool,
) -> Result<Vec<PathBuf>> {
    let files = files.iter().map(|f| f.as_ref()).collect_vec();
    let mut gb = GlobSetBuilder::new();
    for g in glob {
        let mut builder = GlobBuilder::new(g);
        builder.empty_alternates(true);
        if literal_separator {
            builder.literal_separator(true);
        }
        let g = builder.build()?;
        gb.add(g);
    }
    let gs = gb.build()?;
    let matches = files
        .into_iter()
        .filter(|f| gs.is_match(f))
        .map(|f| f.to_path_buf())
        .collect_vec();
    Ok(matches)
}

/// Expands exclude globs that name a directory so they also exclude its contents.
///
/// A pattern whose last path segment has no `*`, `?`, or `[` (`vendor`,
/// `vendor/`, `third_party/vendor`, `**/vendor`) may name a directory, so
/// `<pattern>/*` and `<pattern>/**` are added alongside it. Other patterns are
/// kept as written.
pub fn expand_directory_excludes<I>(patterns: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut expanded = Vec::new();
    for pattern in patterns {
        let pattern = pattern.as_ref();
        expanded.push(pattern.to_string());
        let dir = pattern.trim_end_matches('/');
        let last_segment = dir.rsplit('/').next().unwrap_or(dir);
        if !last_segment.is_empty() && !last_segment.contains(['*', '?', '[']) {
            expanded.push(format!("{dir}/*"));
            expanded.push(format!("{dir}/**"));
        }
    }
    expanded
}

#[cfg(test)]
mod directory_exclude_tests {
    use super::*;

    #[test]
    fn directory_excludes_expand_to_their_contents() {
        assert_eq!(
            expand_directory_excludes(["vendor"]),
            ["vendor", "vendor/*", "vendor/**"]
        );
        assert_eq!(
            expand_directory_excludes(["vendor/"]),
            ["vendor/", "vendor/*", "vendor/**"]
        );
        assert_eq!(
            expand_directory_excludes(["**/vendor"]),
            ["**/vendor", "**/vendor/*", "**/vendor/**"]
        );
        assert_eq!(
            expand_directory_excludes(["src/*/vendor"]),
            ["src/*/vendor", "src/*/vendor/*", "src/*/vendor/**"]
        );
        // Patterns that end in a glob segment are left as written.
        assert_eq!(expand_directory_excludes(["*.snap"]), ["*.snap"]);
        assert_eq!(expand_directory_excludes(["vendor/**"]), ["vendor/**"]);
        assert_eq!(expand_directory_excludes(["**"]), ["**"]);
        assert_eq!(expand_directory_excludes(["/"]), ["/"]);
    }

    #[test]
    fn expanded_excludes_match_directory_contents_at_any_depth() {
        let files = [
            "vendor/a.js",
            "vendor/deep/b.js",
            "src/vendor/c.js",
            "src/main.js",
            "vendors/d.js",
        ];
        let excluded = |pattern: &str, dir: Option<&str>| {
            let expanded = Pattern::Globs(expand_directory_excludes([pattern]));
            get_pattern_matches(&expanded, &files, dir)
                .unwrap()
                .into_iter()
                .map(|f| f.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            excluded("vendor", None),
            ["vendor/a.js", "vendor/deep/b.js"]
        );
        assert_eq!(
            excluded("vendor/", None),
            ["vendor/a.js", "vendor/deep/b.js"]
        );
        assert_eq!(
            excluded("**/vendor", None),
            ["vendor/a.js", "vendor/deep/b.js", "src/vendor/c.js"]
        );
        // Under a step's `dir`, the pattern is relative to it.
        assert_eq!(excluded("vendor", Some("src")), ["src/vendor/c.js"]);
        assert!(excluded("vendor", Some("vendor")).is_empty());
    }
}

pub fn get_pattern_matches<P: AsRef<Path>>(
    pattern: &Pattern,
    files: &[P],
    dir: Option<&str>,
) -> Result<Vec<PathBuf>> {
    // Pre-filter files by dir if specified
    let files_vec: Vec<PathBuf> = if let Some(dir) = dir {
        files
            .iter()
            .map(|f| f.as_ref())
            .filter(|f| f.starts_with(dir))
            .map(|f| f.to_path_buf())
            .collect()
    } else {
        files.iter().map(|f| f.as_ref().to_path_buf()).collect()
    };

    match pattern {
        Pattern::Globs(globs) => {
            // When dir is set, prefix globs with the directory and use strict matching
            if let Some(dir) = dir {
                let dir_globs = globs
                    .iter()
                    .map(|g| format!("{}/{}", dir.trim_end_matches('/'), g))
                    .collect::<Vec<_>>();
                // Use strict matching (literal_separator=true) to ensure proper path semantics
                get_matches_strict(&dir_globs, &files_vec)
            } else {
                get_matches(globs, &files_vec)
            }
        }
        Pattern::Regex { pattern, .. } => {
            let re = Regex::new(pattern)?;
            let matches = files_vec
                .iter()
                .filter(|f| {
                    // For regex patterns, if dir is set, match against the path relative to dir
                    let path_to_match = if let Some(dir) = dir {
                        f.strip_prefix(dir).unwrap_or(f.as_path())
                    } else {
                        f.as_path()
                    };

                    if let Some(path_str) = path_to_match.to_str() {
                        re.is_match(path_str)
                    } else {
                        false
                    }
                })
                .map(|f| f.to_path_buf())
                .collect_vec();
            Ok(matches)
        }
    }
}
