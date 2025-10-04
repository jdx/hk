use crate::Result;
use crate::step::Pattern;
use globset::{GlobBuilder, GlobSetBuilder};
use itertools::Itertools;
use regex::Regex;
use std::path::{Path, PathBuf};

pub fn get_matches<P: AsRef<Path>>(glob: &[String], files: &[P]) -> Result<Vec<PathBuf>> {
    let files = files.iter().map(|f| f.as_ref()).collect_vec();
    let mut gb = GlobSetBuilder::new();
    for g in glob {
        let g = GlobBuilder::new(g).empty_alternates(true).build()?;
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
            // When dir is set, prefix globs with the directory
            if let Some(dir) = dir {
                let dir_globs = globs
                    .iter()
                    .map(|g| format!("{}/{}", dir.trim_end_matches('/'), g))
                    .collect::<Vec<_>>();
                get_matches(&dir_globs, &files_vec)
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
                        f.strip_prefix(dir).unwrap_or(f)
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
