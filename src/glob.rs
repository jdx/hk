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

/// Compile one glob with the options the runtime matchers use.
fn build_glob(glob: &str, literal_separator: bool) -> Result<globset::Glob> {
    let mut builder = GlobBuilder::new(glob);
    builder.empty_alternates(true);
    if literal_separator {
        builder.literal_separator(true);
    }
    Ok(builder.build()?)
}

/// Check that a step pattern compiles, so a bad glob or regex is reported when
/// the config loads instead of when a step first filters files.
pub fn validate_pattern(pattern: &Pattern) -> std::result::Result<(), String> {
    match pattern {
        Pattern::Globs(globs) => {
            for glob in globs {
                // `dir` makes the runtime match strictly, so check that mode too.
                build_glob(glob, false)
                    .and_then(|_| build_glob(glob, true))
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
