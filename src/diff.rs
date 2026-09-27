use similar::TextDiff;

/// Render a unified diff between two strings
///
/// Labels that name paths are quoted the way git quotes them when they
/// contain a control character, `"`, or `\`, so a tab in a file name isn't
/// read back as the start of a timestamp.
pub fn render_unified_diff(old: &str, new: &str, old_label: &str, new_label: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    diff.unified_diff()
        .context_radius(3)
        .header(&quote_path(old_label), &quote_path(new_label))
        .to_string()
}

/// `path` as git writes it in a diff header: unchanged, or C-quoted if it
/// contains a control character, `"`, or `\`. Non-ASCII characters are left
/// as they are, as with git's `core.quotePath = false`.
pub fn quote_path(path: &str) -> std::borrow::Cow<'_, str> {
    if !path
        .chars()
        .any(|c| c.is_control() || c == '"' || c == '\\')
    {
        return path.into();
    }
    let mut quoted = String::with_capacity(path.len() + 2);
    quoted.push('"');
    for c in path.chars() {
        match c {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\t' => quoted.push_str("\\t"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            c if c.is_ascii_control() => quoted.push_str(&format!("\\{:03o}", c as u32)),
            c => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted.into()
}

/// The path in a git C-quoted header side (`"a/foo\tbar"`) and what follows
/// its closing quote, or `None` if `side` isn't quoted.
pub fn unquote_path(side: &str) -> Option<(String, &str)> {
    let rest = side.strip_prefix('"')?;
    let mut bytes = Vec::with_capacity(rest.len());
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((String::from_utf8_lossy(&bytes).into_owned(), &rest[i + 1..])),
            '\\' => {
                let (_, escaped) = chars.next()?;
                let byte = match escaped {
                    'a' => 0x07,
                    'b' => 0x08,
                    't' => b'\t',
                    'n' => b'\n',
                    'v' => 0x0b,
                    'f' => 0x0c,
                    'r' => b'\r',
                    '"' => b'"',
                    '\\' => b'\\',
                    d @ '0'..='7' => {
                        let mut value = d.to_digit(8)?;
                        for _ in 0..2 {
                            let (_, d) = chars.next()?;
                            value = value * 8 + d.to_digit(8)?;
                        }
                        u8::try_from(value).ok()?
                    }
                    _ => return None,
                };
                bytes.push(byte);
            }
            c => {
                let mut buf = [0; 4];
                bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    None
}

#[cfg(test)]
mod quote_path_tests {
    use super::{quote_path, unquote_path};

    #[test]
    fn leaves_ordinary_paths_alone() {
        assert_eq!(quote_path("a/src/main.rs"), "a/src/main.rs");
        assert_eq!(quote_path("a/with space/é.txt"), "a/with space/é.txt");
    }

    #[test]
    fn quotes_and_unquotes_special_characters() {
        let path = "a/foo\tbar\n\"q\"\\x\x07.txt";
        let quoted = quote_path(path);
        assert_eq!(quoted, "\"a/foo\\tbar\\n\\\"q\\\"\\\\x\\007.txt\"");
        assert_eq!(
            unquote_path(&format!("{quoted}\t2026-01-01")),
            Some((path.to_string(), "\t2026-01-01"))
        );
    }

    #[test]
    fn unquoting_needs_a_closing_quote() {
        assert_eq!(unquote_path("\"a/foo"), None);
        assert_eq!(unquote_path("a/foo"), None);
    }
}
