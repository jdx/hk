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
    let files_in_dir: Vec<&Path> = if let Some(dir) = dir {
        files
            .iter()
            .map(|f| f.as_ref())
            .filter(|f| f.starts_with(dir))
            .collect()
    } else {
        files.iter().map(|f| f.as_ref()).collect()
    };

    match pattern {
        Pattern::Globs(globs) => {
            if let Some(dir) = dir {
                // For globs with dir, match against paths relative to dir (like regex)
                // This avoids the double-application of dir context
                let relative_paths: Vec<PathBuf> = files_in_dir
                    .iter()
                    .map(|f| f.strip_prefix(dir).unwrap_or(f).to_path_buf())
                    .collect();

                // Use strict matching to ensure proper path semantics
                let matched_relative = get_matches_strict(globs, &relative_paths)?;

                // Convert back to full paths
                Ok(matched_relative
                    .into_iter()
                    .map(|rel| {
                        let dir_path = Path::new(dir);
                        dir_path.join(rel)
                    })
                    .collect())
            } else {
                // Without dir, match against full paths as before
                let full_paths: Vec<PathBuf> =
                    files_in_dir.iter().map(|f| f.to_path_buf()).collect();
                get_matches(globs, &full_paths)
            }
        }
        Pattern::Regex { pattern, .. } => {
            let re = Regex::new(pattern)?;
            let matches = files_in_dir
                .iter()
                .filter(|f| {
                    // For regex patterns, if dir is set, match against the path relative to dir
                    let path_to_match = if let Some(dir) = dir {
                        f.strip_prefix(dir).unwrap_or(f)
                    } else {
                        f
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
