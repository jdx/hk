use crate::par::PathMemo;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, LazyLock};

/// Cache for file type detection results. Steps that filter the same files at
/// once share one detection per file.
static FILE_TYPE_CACHE: LazyLock<PathMemo<Arc<HashSet<String>>>> = LazyLock::new(PathMemo::new);

/// Get all type tags for a given file path
/// Returns a set of tags like: {"text", "python"}, {"binary", "image", "png"}, etc.
#[cfg(test)]
pub fn get_file_types(path: &Path) -> HashSet<String> {
    cached_file_types(path).as_ref().clone()
}

fn cached_file_types(path: &Path) -> Arc<HashSet<String>> {
    FILE_TYPE_CACHE
        .get_or_try_init(path, || Some(Arc::new(detect_file_types(path))))
        .expect("file type detection always returns a value")
}

fn detect_file_types(path: &Path) -> HashSet<String> {
    let mut types = HashSet::new();

    // 1. Check if it's a symlink (but continue to detect target's type)
    let symlink_metadata = std::fs::symlink_metadata(path).ok();
    let is_symlink = symlink_metadata.as_ref().is_some_and(|m| m.is_symlink());
    if let Some(metadata) = &symlink_metadata {
        crate::step::cache_symlink_check(path, metadata);
    }
    if is_symlink {
        types.insert("symlink".to_string());
    }

    // 2. Check if it's executable (follows symlinks). Only a symlink needs a
    // second stat; if `lstat` failed, `stat` would too.
    let metadata = if is_symlink {
        std::fs::metadata(path).ok()
    } else {
        symlink_metadata
    };
    if let Some(metadata) = metadata {
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

    types
}

/// Check if a file matches any of the given type filters (OR logic)
pub fn matches_types(path: &Path, type_filters: &[String]) -> bool {
    if type_filters.is_empty() {
        return true;
    }

    let file_types = cached_file_types(path);
    type_filters
        .iter()
        .any(|filter| file_types.contains(filter))
}

/// The interpreter a shebang line names: its program's base name, or for
/// `env` the command it runs.
///
/// `env` is followed by its own options and `NAME=value` assignments before
/// the command: `#!/usr/bin/env -S python3 -u`, `#!/usr/bin/env FOO=1 python3`,
/// and `#! /usr/bin/env\tpython3` all run `python3`. Whitespace between the
/// shebang and the program, and tabs between arguments, are allowed.
fn shebang_interpreter(line: &str) -> &str {
    fn base_name(program: &str) -> &str {
        // A quoted command line such as `'python3 -u'` runs its first word
        let program = program.trim_matches(['"', '\'']);
        let program = program.split_whitespace().next().unwrap_or("");
        program.rsplit('/').next().unwrap_or(program)
    }

    let text = line.trim().trim_start_matches("#!");
    let mut tokens = shebang_words(text);
    let program = base_name(tokens.next().unwrap_or(""));
    if program != "env" {
        return program;
    }
    // The text glued to `-S` is the start of the command line env splits.
    let mut pending = None;
    loop {
        let Some(token) = pending.take().or_else(|| tokens.next()) else {
            return "";
        };
        if let Some(command) = split_string_command(token) {
            // GNU env refuses to run a script whose `-S` string has an escape
            // it does not know, so the script has no interpreter to type by.
            if has_invalid_env_escape(text) {
                return "";
            }
            pending = Some(command).filter(|command| !command.is_empty());
        } else if token.starts_with('-') {
            // These options take the next token as their argument.
            if matches!(token, "-u" | "--unset" | "-C" | "--chdir" | "-P") {
                tokens.next();
            }
        } else if !is_env_assignment(token) {
            return base_name(token);
        }
    }
}

/// Splits `text` at whitespace outside quotes, as env does for `-S`, so that
/// `FOO="a b"` stays one word. Quotes are left in the words.
fn shebang_words(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        rest = rest.trim_start();
        if rest.is_empty() {
            return None;
        }
        let mut quote = None;
        let mut escaped = false;
        let mut end = rest.len();
        for (i, c) in rest.char_indices() {
            // A backslash escapes the next character; inside single quotes
            // only a backslash or a quote
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                let next = rest[i + 1..].chars().next();
                if quote != Some('\'') || matches!(next, Some('\\' | '\'')) {
                    escaped = true;
                    continue;
                }
            }
            match quote {
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '"' || c == '\'' => quote = Some(c),
                None if c.is_whitespace() => {
                    end = i;
                    break;
                }
                None => {}
            }
        }
        let (word, remainder) = rest.split_at(end);
        rest = remainder;
        Some(word)
    })
}

/// Whether `text`, split by env's `-S`, has a backslash escape GNU env rejects.
/// Outside single quotes it knows `\c \f \n \r \t \v \_ \# \$ \" \\`; inside
/// them only `\\` and `\'` are escapes, and other backslashes are literal.
fn has_invalid_env_escape(text: &str) -> bool {
    let mut quote = None;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (Some('\''), '\\') => {
                if matches!(chars.peek(), Some('\\' | '\'')) {
                    chars.next();
                }
            }
            (_, '\\') => match chars.next() {
                Some('c' | 'f' | 'n' | 'r' | 't' | 'v' | '_' | '#' | '$' | '"' | '\\') => {}
                _ => return true,
            },
            _ => {}
        }
    }
    false
}

/// If `token` is env's `-S` / `--split-string` option, the start of the command
/// line that is glued to it (empty when the command is the next token). A
/// cluster such as `-iS` counts.
fn split_string_command(token: &str) -> Option<&str> {
    if let Some(rest) = token.strip_prefix("--split-string") {
        return match rest.strip_prefix('=') {
            Some(command) => Some(command),
            None if rest.is_empty() => Some(""),
            None => None,
        };
    }
    let flags = token.strip_prefix('-').filter(|f| !f.starts_with('-'))?;
    let at = flags.find('S')?;
    // Only flags that take no argument may precede the `S`.
    flags[..at]
        .chars()
        .all(|c| matches!(c, 'i' | 'v' | '0'))
        .then(|| &flags[at + 1..])
}

/// Whether `token` is a `NAME=value` assignment that `env` applies.
fn is_env_assignment(token: &str) -> bool {
    token.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty()
            && !name.starts_with(|c: char| c.is_ascii_digit())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
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

    let interpreter = shebang_interpreter(&first_line);

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

    // Try magic number detection first. infer also sniffs a few text formats
    // (HTML, XML, shebang scripts); those fall through to the null-byte scan.
    let kind = infer::get_from_path(path).ok().flatten();
    if let Some(kind) = kind.filter(|k| k.matcher_type() != infer::MatcherType::Text) {
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
        // Only the XML prolog is specific enough to tag. HTML sniffing also
        // matches Svelte, Vue, and other templates that open with a tag.
        if kind.is_some_and(|k| k.mime_type() == "text/xml") {
            types.insert("xml".to_string());
        }
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
        "adoc" => {
            types.insert("text".to_string());
            types.insert("asciidoc".to_string());
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

        // Frontend component formats
        "svelte" => {
            types.insert("text".to_string());
            types.insert("svelte".to_string());
        }
        "vue" => {
            types.insert("text".to_string());
            types.insert("vue".to_string());
        }
        "astro" => {
            types.insert("text".to_string());
            types.insert("astro".to_string());
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
    fn shebang_interpreter_skips_env_options_and_assignments() {
        for (line, expected) in [
            ("#!/bin/sh\n", "sh"),
            ("#!/usr/bin/python3 -u\n", "python3"),
            ("#!/usr/bin/env python3\n", "python3"),
            ("#! /usr/bin/env python3\n", "python3"),
            ("#!/usr/bin/env\tpython3\n", "python3"),
            ("#!/usr/bin/env -S python3 -u\n", "python3"),
            ("#!/usr/bin/env -S\tpython3 -u\n", "python3"),
            ("#!/usr/bin/env -Spython3 -u\n", "python3"),
            ("#!/usr/bin/env --split-string=python3 -u\n", "python3"),
            ("#!/usr/bin/env --split-string python3\n", "python3"),
            ("#!/usr/bin/env -iS python3\n", "python3"),
            ("#!/usr/bin/env -S 'python3 -u'\n", "python3"),
            ("#!/usr/bin/env FOO=bar python3\n", "python3"),
            ("#!/usr/bin/env -S FOO=bar BAZ=1 python3 -u\n", "python3"),
            ("#!/usr/bin/env -S FOO=\"a b\" python3\n", "python3"),
            // An escaped quote does not end a quoted value; an escaped
            // backslash does not escape the closing quote
            ("#!/usr/bin/env -S FOO=\"a\\\" b\" python3\n", "python3"),
            ("#!/usr/bin/env -S FOO=\"a\\\\\" python3\n", "python3"),
            // Backslashes are literal inside single quotes
            ("#!/usr/bin/env -S FOO='a\\b' python3\n", "python3"),
            ("#!/usr/bin/env -S FOO='a\\\\' python3\n", "python3"),
            ("#!/usr/bin/env -S FOO=\"a\\$b\" python3\n", "python3"),
            // GNU env rejects other escapes, such as a backslash and a space,
            // so the script cannot start and has no interpreter
            ("#!/usr/bin/env -S FOO=a\\ b python3\n", ""),
            ("#!/usr/bin/env -S FOO=\"a\\qb\" python3\n", ""),
            ("#!/usr/bin/env -S FOO='a b' BAR=\"c  d\" ruby -w\n", "ruby"),
            ("#!/usr/bin/env -u HOME -i ruby\n", "ruby"),
            ("#!/usr/bin/env -C /tmp node\n", "node"),
            ("#!/bin/env bash\n", "bash"),
            ("#!/usr/bin/env\n", ""),
            ("#!/usr/bin/env -S\n", ""),
        ] {
            assert_eq!(shebang_interpreter(line), expected, "{line:?}");
        }
    }

    #[test]
    fn env_shebang_variants_are_typed_by_their_interpreter() {
        for (shebang, tag) in [
            ("#!/usr/bin/env -S python3 -u", "python"),
            ("#! /usr/bin/env python3", "python"),
            ("#!/usr/bin/env\tbash", "bash"),
            ("#!/usr/bin/env FOO=1 ruby", "ruby"),
            (
                "#!/usr/bin/env -S node --experimental-strip-types",
                "javascript",
            ),
        ] {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(format!("{shebang}\nbody\n").as_bytes())
                .unwrap();
            let types = detect_shebang(file.path()).unwrap();
            assert!(types.contains(tag), "{shebang:?}: {types:?}");
        }
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
    fn test_frontend_component_extensions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let fixtures = [
            ("App.svelte", "svelte", "<script lang=\"ts\">\n</script>\n"),
            ("App.vue", "vue", "<script setup>\n</script>\n"),
            ("Page.astro", "astro", "---\nconst title = 'hi';\n---\n"),
        ];
        let mut files = Vec::new();
        for (name, _, content) in fixtures {
            let path = temp_dir.path().join(name);
            std::fs::write(&path, content).unwrap();
            files.push(path);
        }
        let unrelated = temp_dir.path().join("util.ts");
        std::fs::write(&unrelated, "export const value = 1;\n").unwrap();
        files.push(unrelated);

        for (name, tag, _) in fixtures {
            let path = temp_dir.path().join(name);
            let types = get_file_types(&path);
            assert!(types.contains("text"), "{name}: got {types:?}");
            assert!(types.contains(tag), "{name}: got {types:?}");
            assert!(!types.contains("binary"), "{name}: got {types:?}");
            assert!(!types.contains("html"), "{name}: got {types:?}");

            let step = crate::step::Step {
                types: Some(vec![tag.to_string()]),
                ..Default::default()
            };
            assert_eq!(step.filter_files(&files).unwrap(), vec![path], "{name}");
        }
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

    #[test]
    fn test_svelte_component_is_text() {
        // Svelte components start with `<script`, which HTML sniffing matches
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("Component.svelte");
        std::fs::write(
            &path,
            b"<script lang=\"ts\">\n  let name = 'world';\n</script>\n\n<h1>Hello {name}!</h1>\n",
        )
        .unwrap();

        let types = get_file_types(&path);
        assert!(types.contains("text"), "got {types:?}");
        assert!(!types.contains("binary"), "got {types:?}");
    }

    #[test]
    fn test_unknown_extension_with_html_like_content_is_text() {
        let temp_dir = tempfile::tempdir().unwrap();
        for (name, content) in [
            (
                "script.tmpl",
                "<script lang=\"ts\">\n  let n = 1;\n</script>\n",
            ),
            ("comment.tmpl", "<!-- header partial -->\n<nav></nav>\n"),
            ("div.tmpl", "<div>\n  {{ content }}\n</div>\n"),
        ] {
            let path = temp_dir.path().join(name);
            std::fs::write(&path, content).unwrap();

            let types = get_file_types(&path);
            assert!(types.contains("text"), "{name}: got {types:?}");
            assert!(!types.contains("binary"), "{name}: got {types:?}");
            assert!(!types.contains("html"), "{name}: got {types:?}");
        }
    }

    #[test]
    fn test_unknown_extension_with_xml_prolog_is_xml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("App.csproj");
        std::fs::write(
            &path,
            b"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<Project Sdk=\"Microsoft.NET.Sdk\" />\n",
        )
        .unwrap();

        let types = get_file_types(&path);
        assert!(types.contains("text"), "got {types:?}");
        assert!(types.contains("xml"), "got {types:?}");
        assert!(!types.contains("binary"), "got {types:?}");
    }

    #[test]
    fn test_text_signature_with_null_byte_is_binary() {
        let temp_dir = tempfile::tempdir().unwrap();
        for (name, content) in [
            (
                "script.tmpl",
                "<script lang=\"ts\">\n\0let n = 1;\n</script>\n",
            ),
            (
                "App.csproj",
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\0<Project />\n",
            ),
        ] {
            let path = temp_dir.path().join(name);
            std::fs::write(&path, content).unwrap();

            // Ensure the fixture reaches the text-matcher null-byte fallback.
            let kind = infer::get_from_path(&path).unwrap().unwrap();
            assert_eq!(kind.matcher_type(), infer::MatcherType::Text, "{name}");

            let types = get_file_types(&path);
            assert!(types.contains("binary"), "{name}: got {types:?}");
            for tag in ["text", "html", "xml"] {
                assert!(!types.contains(tag), "{name}: got {types:?}");
            }
        }
    }

    #[test]
    fn test_png_content_without_extension_is_binary() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("image");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").unwrap();

        let types = get_file_types(&path);
        assert!(types.contains("binary"), "got {types:?}");
        assert!(types.contains("image"), "got {types:?}");
        assert!(types.contains("png"), "got {types:?}");
        assert!(!types.contains("text"), "got {types:?}");
    }
}
