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
            // When dir is set, match the globs against paths relative to it,
            // with strict matching (literal_separator=true) to ensure proper
            // path semantics. Prefixing the globs with `dir` instead would
            // read glob metacharacters in the directory's name (`[`, `{`) as
            // part of the pattern.
            if let Some(dir) = dir {
                let relative = files_vec
                    .iter()
                    .filter_map(|f| {
                        let relative = f.strip_prefix(dir).ok()?;
                        // A file named like the directory itself is not inside it.
                        (!relative.as_os_str().is_empty())
                            .then(|| (relative.to_path_buf(), f.clone()))
                    })
                    .collect_vec();
                let matched =
                    get_matches_strict(globs, &relative.iter().map(|(rel, _)| rel).collect_vec())?;
                let matched: std::collections::HashSet<&Path> =
                    matched.iter().map(PathBuf::as_path).collect();
                Ok(relative
                    .iter()
                    .filter(|(rel, _)| matched.contains(rel.as_path()))
                    .map(|(_, full)| full.clone())
                    .collect())
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
