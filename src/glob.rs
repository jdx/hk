use crate::Result;
use globset::{Glob, GlobSetBuilder};
use itertools::Itertools;
use miette::IntoDiagnostic;
use std::path::{Path, PathBuf};

pub fn get_matches<P: AsRef<Path>>(glob: &[String], staged_files: &[P]) -> Result<Vec<PathBuf>> {
    let staged_files = staged_files.iter().map(|f| f.as_ref()).collect_vec();
    let mut gb = GlobSetBuilder::new();
    for g in glob {
        gb.add(Glob::new(g).into_diagnostic()?);
    }
    let gs = gb.build().into_diagnostic()?;
    let matches = staged_files
        .into_iter()
        .filter(|f| gs.is_match(f))
        .map(|f| f.to_path_buf())
        .collect_vec();
    Ok(matches)
}
