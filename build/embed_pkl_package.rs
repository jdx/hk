use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Modules the package must contain to be usable as a drop-in replacement for
/// the released archive. Embedding a partial package would poison the pkl
/// cache: an import of a missing module fails outright instead of falling back
/// to the network.
const REQUIRED_MODULES: &[&str] = &["Builtins.pkl", "Config.pkl", "Types.pkl", "UserConfig.pkl"];

/// Files `pkl project package` keeps out of the published archive.
const EXCLUDED: &[&str] = &["PklProject", "PklProject.deps.json", ".gitignore"];

/// Stage a zip of the pkl sources mirroring the released `hk@VERSION.zip`, so
/// the binary can seed the pkl cache with the package matching its own version.
///
/// Writes an empty file when the generated sources are absent (`pkl:gen` has
/// not run); hk then treats the package as unavailable and fetches normally.
pub fn generate(out_dir: &Path) -> Result<(), std::io::Error> {
    let pkl_dir = Path::new("pkl");
    let dest_path = out_dir.join("hk_pkl_package.zip");

    let entries = collect_entries(pkl_dir)?;
    let complete = REQUIRED_MODULES
        .iter()
        .all(|module| entries.iter().any(|(name, _)| name == module));
    if !complete {
        fs::write(&dest_path, [])?;
        return Ok(());
    }

    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    // A fixed timestamp keeps the archive byte-identical across rebuilds of the
    // same sources, so the embedded bytes do not churn the binary.
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for (name, path) in &entries {
        zip.start_file(name, options)
            .map_err(std::io::Error::other)?;
        zip.write_all(&fs::read(path)?)?;
    }
    let archive = zip.finish().map_err(std::io::Error::other)?.into_inner();
    fs::write(&dest_path, archive)?;
    Ok(())
}

/// Collect the archive entries as (name within the archive, source path),
/// sorted so the archive order does not depend on directory iteration order.
fn collect_entries(pkl_dir: &Path) -> Result<Vec<(String, PathBuf)>, std::io::Error> {
    let mut entries = Vec::new();
    collect_dir(pkl_dir, "", &mut entries)?;
    entries.sort();
    Ok(entries)
}

/// Add the regular files under `dir` to `entries`, recursing into
/// subdirectories and prefixing each name with its path inside the archive.
fn collect_dir(
    dir: &Path,
    prefix: &str,
    entries: &mut Vec<(String, PathBuf)>,
) -> Result<(), std::io::Error> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        // Archive paths always use forward slashes, on every platform.
        let name = format!("{prefix}{file_name}");
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_dir(&entry.path(), &format!("{name}/"), entries)?;
        } else if file_type.is_symlink() {
            // Reading through a link would embed whatever it points at on the
            // build machine. Fail rather than skip: a silently omitted module
            // would ship a partial package and break the imports that need it.
            return Err(std::io::Error::other(format!(
                "refusing to embed symbolic link: {}",
                entry.path().display()
            )));
        } else if file_type.is_file() && !EXCLUDED.contains(&name.as_str()) {
            entries.push((name, entry.path()));
        }
    }
    Ok(())
}
