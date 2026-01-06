use dashmap::DashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// Cache for file type detection results
static FILE_TYPE_CACHE: LazyLock<DashMap<PathBuf, HashSet<String>>> = LazyLock::new(DashMap::new);

/// Get all type tags for a given file path
/// Returns a set of tags like: {"text", "python"}, {"binary", "image", "png"}, etc.
pub fn get_file_types(path: &Path) -> HashSet<String> {
    // Check cache first
    if let Some(types) = FILE_TYPE_CACHE.get(path) {
        return types.clone();
    }

    let mut types = HashSet::new();

    // 1. Check if it's a symlink (but continue to detect target's type)
    if let Ok(metadata) = std::fs::symlink_metadata(path)
        && metadata.is_symlink()
    {
        types.insert("symlink".to_string());
    }

    // 2. Check if it's executable (follows symlinks)
    if let Ok(metadata) = std::fs::metadata(path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 != 0 {
                types.insert("executable".to_string());
            }
        }
    }

    // 3. For symlinks, also check the target's filename and extension
    // For non-symlinks, use the path directly
    let check_path = if types.contains("symlink") {
        std::fs::read_link(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };

    // 4. Check by filename (e.g., Dockerfile, Makefile)
    if let Some(filename) = check_path.file_name().and_then(|n| n.to_str())
        && let Some(name_types) = get_types_by_filename(filename)
    {
        types.extend(name_types);
    }

    // 5. Check by extension
    if let Some(ext) = check_path.extension().and_then(|e| e.to_str())
        && let Some(ext_types) = get_types_by_extension(ext)
    {
        types.extend(ext_types);
    }

    // 6. Check shebang for executable text files
    if (types.contains("executable") || types.is_empty())
        && let Some(shebang_types) = detect_shebang(path)
    {
        types.extend(shebang_types);
    }

    // 7. Check magic number / content-based detection
    if (types.is_empty() || !types.contains("text"))
        && let Some(content_types) = detect_by_content(path)
    {
        types.extend(content_types);
    }

    // 8. If still no type detected, default to text if not binary
    if types.is_empty() {
        types.insert("text".to_string());
    }

    FILE_TYPE_CACHE.insert(path.to_path_buf(), types.clone());
    types
}

/// Check if a file matches any of the given type filters (OR logic)
pub fn matches_types(path: &Path, type_filters: &[String]) -> bool {
    if type_filters.is_empty() {
        return true;
    }

    let file_types = get_file_types(path);
    type_filters
        .iter()
        .any(|filter| file_types.contains(filter))
}

/// Detect file types by reading shebang line
fn detect_shebang(path: &Path) -> Option<HashSet<String>> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut first_line = String::new();
    reader.read_line(&mut first_line).ok()?;

    if !first_line.starts_with("#!") {
        return None;
    }

    let mut types = HashSet::new();
    types.insert("text".to_string());

    let shebang = first_line.trim();

    // Handle /usr/bin/env cases
    let interpreter = if shebang.contains("/env ") {
        shebang
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .split('/')
            .next_back()
            .unwrap_or("")
    } else {
        shebang
            .trim_start_matches("#!")
            .split_whitespace()
            .next()
            .unwrap_or("")
            .split('/')
            .next_back()
            .unwrap_or("")
    };

    match interpreter {
        s if s.starts_with("python") => {
            types.insert("python".to_string());
        }
        s if s.starts_with("node") || s == "nodejs" => {
            types.insert("javascript".to_string());
            types.insert("node".to_string());
        }
        s if s.starts_with("ruby") => {
            types.insert("ruby".to_string());
        }
        "sh" | "dash" => {
            types.insert("shell".to_string());
            types.insert("sh".to_string());
        }
        "bash" => {
            types.insert("shell".to_string());
            types.insert("bash".to_string());
        }
        "zsh" => {
            types.insert("shell".to_string());
            types.insert("zsh".to_string());
        }
        "fish" => {
            types.insert("shell".to_string());
            types.insert("fish".to_string());
        }
        "perl" => {
            types.insert("perl".to_string());
        }
        "php" => {
            types.insert("php".to_string());
        }
        _ => {}
    }

    Some(types)
}

/// Detect file types by reading content/magic numbers
fn detect_by_content(path: &Path) -> Option<HashSet<String>> {
    let mut types = HashSet::new();

    // Try magic number detection first
    if let Ok(Some(kind)) = infer::get_from_path(path) {
        types.insert("binary".to_string());

        // Map infer's MIME types to our type tags
        let mime = kind.mime_type();
        match mime {
            // Images
            m if m.starts_with("image/") => {
                types.insert("image".to_string());
                if let Some(subtype) = m.strip_prefix("image/") {
                    types.insert(subtype.to_string());
                }
            }
            // Videos
            m if m.starts_with("video/") => {
                types.insert("video".to_string());
            }
            // Audio
            m if m.starts_with("audio/") => {
                types.insert("audio".to_string());
            }
            // Archives
            "application/zip" => {
                types.insert("archive".to_string());
                types.insert("zip".to_string());
            }
            "application/gzip" | "application/x-gzip" => {
                types.insert("archive".to_string());
                types.insert("gzip".to_string());
            }
            "application/x-tar" => {
                types.insert("archive".to_string());
                types.insert("tar".to_string());
            }
            // PDFs
            "application/pdf" => {
                types.insert("pdf".to_string());
            }
            _ => {}
        }

        return Some(types);
    }

    // If no magic number found, fallback to null-byte scanning
    use std::io::Read;
    let mut file = File::open(path).ok()?;
    let mut buffer = [0u8; 8192];
    let bytes_read = file.read(&mut buffer).ok()?;
    let is_binary = buffer[..bytes_read].contains(&0);

    if is_binary {
        types.insert("binary".to_string());
    } else {
        types.insert("text".to_string());
    }

    Some(types)
}

/// Get types based on file extension
fn get_types_by_extension(ext: &str) -> Option<HashSet<String>> {
    let mut types = HashSet::new();

    match ext.to_lowercase().as_str() {
        // Programming languages
        "py" | "pyw" => {
            types.insert("text".to_string());
            types.insert("python".to_string());
        }
        "pyi" => {
            types.insert("text".to_string());
            types.insert("python".to_string());
            types.insert("pyi".to_string());
        }
        "js" | "mjs" | "cjs" => {
            types.insert("text".to_string());
            types.insert("javascript".to_string());
        }
        "jsx" => {
            types.insert("text".to_string());
            types.insert("javascript".to_string());
            types.insert("jsx".to_string());
        }
        "ts" | "mts" | "cts" => {
            types.insert("text".to_string());
            types.insert("typescript".to_string());
        }
        "tsx" => {
            types.insert("text".to_string());
            types.insert("typescript".to_string());
            types.insert("tsx".to_string());
        }
        "rs" => {
            types.insert("text".to_string());
            types.insert("rust".to_string());
        }
        "go" => {
            types.insert("text".to_string());
            types.insert("go".to_string());
        }
        "rb" => {
            types.insert("text".to_string());
            types.insert("ruby".to_string());
        }
        "php" => {
            types.insert("text".to_string());
            types.insert("php".to_string());
        }
        "java" => {
            types.insert("text".to_string());
            types.insert("java".to_string());
        }
        "kt" | "kts" => {
            types.insert("text".to_string());
            types.insert("kotlin".to_string());
        }
        "swift" => {
            types.insert("text".to_string());
            types.insert("swift".to_string());
        }
        "c" => {
            types.insert("text".to_string());
            types.insert("c".to_string());
        }
        "h" => {
            types.insert("text".to_string());
            types.insert("c".to_string());
            types.insert("header".to_string());
        }
        "cpp" | "cc" | "cxx" | "c++" => {
            types.insert("text".to_string());
            types.insert("c++".to_string());
        }
        "hpp" | "hxx" | "h++" => {
            types.insert("text".to_string());
            types.insert("c++".to_string());
            types.insert("header".to_string());
        }
        "cs" => {
            types.insert("text".to_string());
            types.insert("csharp".to_string());
        }
        "lua" => {
            types.insert("text".to_string());
            types.insert("lua".to_string());
        }
        "sh" | "bash" => {
            types.insert("text".to_string());
            types.insert("shell".to_string());
            types.insert("bash".to_string());
        }
        "zsh" => {
            types.insert("text".to_string());
            types.insert("shell".to_string());
            types.insert("zsh".to_string());
        }
        "fish" => {
            types.insert("text".to_string());
            types.insert("shell".to_string());
            types.insert("fish".to_string());
        }

        // Data formats
        "json" => {
            types.insert("text".to_string());
            types.insert("json".to_string());
        }
        "json5" | "jsonc" => {
            types.insert("text".to_string());
            types.insert("json".to_string());
            types.insert(ext.to_string());
        }
        "yaml" | "yml" => {
            types.insert("text".to_string());
            types.insert("yaml".to_string());
        }
        "toml" => {
            types.insert("text".to_string());
            types.insert("toml".to_string());
        }
        "xml" => {
            types.insert("text".to_string());
            types.insert("xml".to_string());
        }
        "csv" => {
            types.insert("text".to_string());
            types.insert("csv".to_string());
        }
        "pkl" => {
            types.insert("text".to_string());
            types.insert("pkl".to_string());
        }

        // Markup and documentation
        "md" | "markdown" => {
            types.insert("text".to_string());
            types.insert("markdown".to_string());
        }
        "rst" => {
            types.insert("text".to_string());
            types.insert("rst".to_string());
        }
        "html" | "htm" => {
            types.insert("text".to_string());
            types.insert("html".to_string());
        }
        "css" => {
            types.insert("text".to_string());
            types.insert("css".to_string());
        }
        "scss" | "sass" => {
            types.insert("text".to_string());
            types.insert("css".to_string());
            types.insert(ext.to_string());
        }
        "less" => {
            types.insert("text".to_string());
            types.insert("css".to_string());
            types.insert("less".to_string());
        }

        // Config files
        "ini" | "cfg" | "conf" => {
            types.insert("text".to_string());
            types.insert("ini".to_string());
        }

        // Images
        "png" => {
            types.insert("binary".to_string());
            types.insert("image".to_string());
            types.insert("png".to_string());
        }
        "jpg" | "jpeg" => {
            types.insert("binary".to_string());
            types.insert("image".to_string());
            types.insert("jpeg".to_string());
        }
        "gif" => {
            types.insert("binary".to_string());
            types.insert("image".to_string());
            types.insert("gif".to_string());
        }
        "svg" => {
            types.insert("text".to_string());
            types.insert("image".to_string());
            types.insert("svg".to_string());
            types.insert("xml".to_string());
        }
        "webp" => {
            types.insert("binary".to_string());
            types.insert("image".to_string());
            types.insert("webp".to_string());
        }

        // Archives
        "zip" => {
            types.insert("binary".to_string());
            types.insert("archive".to_string());
            types.insert("zip".to_string());
        }
        "tar" => {
            types.insert("binary".to_string());
            types.insert("archive".to_string());
            types.insert("tar".to_string());
        }
        "gz" | "gzip" => {
            types.insert("binary".to_string());
            types.insert("archive".to_string());
            types.insert("gzip".to_string());
        }
        "bz2" => {
            types.insert("binary".to_string());
            types.insert("archive".to_string());
            types.insert("bzip2".to_string());
        }
        "xz" => {
            types.insert("binary".to_string());
            types.insert("archive".to_string());
            types.insert("xz".to_string());
        }

        _ => return None,
    }

    Some(types)
}

/// Get types based on specific filenames
fn get_types_by_filename(filename: &str) -> Option<HashSet<String>> {
    let mut types = HashSet::new();

    match filename.to_lowercase().as_str() {
        "dockerfile" => {
            types.insert("text".to_string());
            types.insert("dockerfile".to_string());
        }
        "makefile" => {
            types.insert("text".to_string());
            types.insert("makefile".to_string());
        }
        "rakefile" => {
            types.insert("text".to_string());
            types.insert("ruby".to_string());
            types.insert("rakefile".to_string());
        }
        "gemfile" => {
            types.insert("text".to_string());
            types.insert("ruby".to_string());
        }
        "cargo.toml" | "cargo.lock" => {
            types.insert("text".to_string());
            types.insert("toml".to_string());
            types.insert("rust".to_string());
        }
        "package.json" | "package-lock.json" => {
            types.insert("text".to_string());
            types.insert("json".to_string());
            types.insert("javascript".to_string());
        }
        "go.mod" | "go.sum" => {
            types.insert("text".to_string());
            types.insert("go".to_string());
        }
        _ => {
            // Check for Dockerfile variants (Dockerfile.dev, etc.)
            if filename.starts_with("Dockerfile") || filename.starts_with("dockerfile") {
                types.insert("text".to_string());
                types.insert("dockerfile".to_string());
            } else {
                return None;
            }
        }
    }

    Some(types)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_python_extension() {
        let mut file = NamedTempFile::new().unwrap();
        let path = file.path().with_extension("py");
        file.write_all(b"print('hello')").unwrap();
        std::fs::rename(file.path(), &path).unwrap();

        let types = get_file_types(&path);
        assert!(types.contains("text"));
        assert!(types.contains("python"));
    }

    #[test]
    fn test_python_shebang() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"#!/usr/bin/env python3\nprint('hello')")
            .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o755);
            file.as_file().set_permissions(perms).unwrap();
        }

        let types = get_file_types(file.path());
        assert!(types.contains("text"));
        assert!(types.contains("python"));
        #[cfg(unix)]
        assert!(types.contains("executable"));
    }

    #[test]
    fn test_shell_shebang() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"#!/bin/bash\necho hello").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.as_file().metadata().unwrap().permissions();
            perms.set_mode(0o755);
            file.as_file().set_permissions(perms).unwrap();
        }

        let types = get_file_types(file.path());
        assert!(types.contains("text"));
        assert!(types.contains("shell"));
        assert!(types.contains("bash"));
    }

    #[test]
    fn test_matches_types_or_logic() {
        let mut file = NamedTempFile::new().unwrap();
        let path = file.path().with_extension("py");
        file.write_all(b"print('hello')").unwrap();
        std::fs::rename(file.path(), &path).unwrap();

        // Should match if ANY type matches
        assert!(matches_types(
            &path,
            &["python".to_string(), "ruby".to_string()]
        ));
        assert!(matches_types(&path, &["text".to_string()]));
        assert!(!matches_types(&path, &["ruby".to_string()]));
    }

    #[test]
    fn test_dockerfile() {
        let mut file = NamedTempFile::new().unwrap();
        let path = file.path().parent().unwrap().join("Dockerfile");
        file.write_all(b"FROM ubuntu:20.04").unwrap();
        std::fs::rename(file.path(), &path).unwrap();

        let types = get_file_types(&path);
        assert!(types.contains("text"));
        assert!(types.contains("dockerfile"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_text_file_without_extension() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"This is a plain text file\nwithout any extension\n")
            .unwrap();

        let types = get_file_types(file.path());
        assert!(types.contains("text"));
        assert!(!types.contains("binary"));
    }

    #[test]
    fn test_binary_file_detection() {
        let mut file = NamedTempFile::new().unwrap();
        // Write some binary data with null bytes
        file.write_all(&[0x00, 0x01, 0x02, 0xFF, 0xFE]).unwrap();

        let types = get_file_types(file.path());
        assert!(types.contains("binary"));
        assert!(!types.contains("text"));
    }

    #[test]
    #[cfg(unix)]
    fn test_symlink_to_python_file() {
        use std::os::unix::fs::symlink;

        // Create target Python file
        let temp_dir = tempfile::tempdir().unwrap();
        let target_path = temp_dir.path().join("target.py");
        std::fs::write(&target_path, b"print('hello')").unwrap();

        // Create a symlink to the Python file
        let link_path = temp_dir.path().join("link_to_script");
        symlink(&target_path, &link_path).unwrap();

        let types = get_file_types(&link_path);
        assert!(types.contains("symlink"), "Should contain symlink type");
        assert!(types.contains("python"), "Should contain python type");
        assert!(types.contains("text"), "Should contain text type");
    }

    #[test]
    #[cfg(unix)]
    fn test_symlink_matches_target_type() {
        use std::os::unix::fs::symlink;

        // Create target Python file
        let temp_dir = tempfile::tempdir().unwrap();
        let target_path = temp_dir.path().join("script.py");
        std::fs::write(&target_path, b"print('hello')").unwrap();

        let link_path = temp_dir.path().join("link_to_script");
        symlink(&target_path, &link_path).unwrap();

        // Should match python type filter even though it's a symlink
        assert!(matches_types(&link_path, &["python".to_string()]));
        assert!(matches_types(&link_path, &["symlink".to_string()]));
    }
}
