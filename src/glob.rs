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
