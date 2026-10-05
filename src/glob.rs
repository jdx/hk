use crate::Result;
use crate::step::Pattern;
use globset::{GlobBuilder, GlobSetBuilder};
use itertools::Itertools;
use regex::Regex;
use std::path::{Path, PathBuf};

pub fn get_matches<P: AsRef<Path>>(glob: &[String], files: &[P]) -> Result<Vec<PathBuf>> {
    get_matches_with_options(glob, files, false)
}

/// Compile one glob with the options the runtime matchers use.
fn build_glob(glob: &str, literal_separator: bool) -> Result<globset::Glob> {
    let mut builder = GlobBuilder::new(glob);
    builder.empty_alternates(true);
    if literal_separator {
        builder.literal_separator(true);
    }
    Ok(builder.build()?)
}

/// Check that a step pattern compiles exactly as `get_pattern_matches` will
/// compile it, so a bad glob or regex is reported when the config loads instead
/// of when a step first filters files.
pub fn validate_pattern(pattern: &Pattern, dir: Option<&str>) -> std::result::Result<(), String> {
    match pattern {
        Pattern::Globs(globs) => {
            // With a dir, globs match paths relative to it, strictly.
            for glob in globs {
                build_glob(glob, dir.is_some())
                    .map_err(|e| format!("invalid glob '{glob}': {e}"))?;
            }
        }
        Pattern::Regex { pattern, .. } => {
            Regex::new(pattern).map_err(|e| format!("invalid regex '{pattern}': {e}"))?;
        }
    }
    Ok(())
}

fn get_matches_with_options<P: AsRef<Path>>(
    glob: &[String],
    files: &[P],
    literal_separator: bool,
) -> Result<Vec<PathBuf>> {
    let files = files.iter().map(|f| f.as_ref()).collect_vec();
    let mut gb = GlobSetBuilder::new();
    for g in glob {
        gb.add(build_glob(g, literal_separator)?);
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

pub fn get_pattern_matches<P: AsRef<Path>>(
    pattern: &Pattern,
    files: &[P],
    dir: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let files = files.iter().map(|f| f.as_ref());
    match pattern {
        Pattern::Globs(globs) => {
            // With a dir, match each file's path relative to it with strict
            // matching (literal_separator=true) to ensure proper path
            // semantics. Prefixing the globs with `dir` instead would read
            // glob metacharacters in the directory's name (`[`, `{`) as part
            // of the pattern.
            let mut gb = GlobSetBuilder::new();
            for g in globs {
                gb.add(build_glob(g, dir.is_some())?);
            }
            let set = gb.build()?;
            Ok(files
                .filter(|f| match dir {
                    // A file named like the directory itself is not inside it.
                    Some(dir) => f
                        .strip_prefix(dir)
                        .is_ok_and(|rel| !rel.as_os_str().is_empty() && set.is_match(rel)),
                    None => set.is_match(f),
                })
                .map(Path::to_path_buf)
                .collect())
        }
        Pattern::Regex { pattern, .. } => {
            let re = Regex::new(pattern)?;
            Ok(files
                .filter_map(|f| {
                    // For regex patterns, if dir is set, match against the path relative to dir
                    let path_to_match = match dir {
                        Some(dir) => f.strip_prefix(dir).ok()?,
                        None => f,
                    };
                    re.is_match(path_to_match.to_str()?)
                        .then(|| f.to_path_buf())
                })
                .collect())
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn globs(patterns: &[&str]) -> Pattern {
        Pattern::Globs(patterns.iter().map(|p| p.to_string()).collect())
    }

    fn matches(patterns: &[&str], files: &[&str], dir: Option<&str>) -> Vec<String> {
        get_pattern_matches(&globs(patterns), files, dir)
            .unwrap()
            .into_iter()
            .map(|f| f.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn dir_scoped_globs_match_relative_to_the_dir() {
        let files = [
            "src/a.rs",
            "src/sub/b.rs",
            "src/c.txt",
            "other/d.rs",
            "srcx/e.rs",
        ];
        assert_eq!(matches(&["*.rs"], &files, Some("src")), ["src/a.rs"]);
        assert_eq!(
            matches(&["**/*.rs"], &files, Some("src")),
            ["src/a.rs", "src/sub/b.rs"]
        );
        assert_eq!(matches(&["*.rs"], &files, Some("src/")), ["src/a.rs"]);
    }

    #[test]
    fn dir_names_with_glob_metacharacters_are_literal() {
        let files = ["app[1]/a.txt", "app1/a.txt", "{x,y}/a.txt", "x/a.txt"];
        assert_eq!(
            matches(&["*.txt"], &files, Some("app[1]")),
            ["app[1]/a.txt"]
        );
        assert_eq!(matches(&["*.txt"], &files, Some("{x,y}")), ["{x,y}/a.txt"]);
    }

    #[test]
    fn a_path_equal_to_the_dir_is_not_inside_it() {
        assert!(matches(&["*"], &["src"], Some("src")).is_empty());
    }

    #[test]
    fn unscoped_globs_keep_loose_separator_matching() {
        assert_eq!(
            matches(&["*.rs"], &["a.rs", "sub/b.rs"], None),
            ["a.rs", "sub/b.rs"]
        );
    }
}
