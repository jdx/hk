use std::{path::Path, sync::LazyLock};

use crate::{Result, git_util, step::ShellType};
use itertools::Itertools;
use serde::Serialize;
use tera::Tera;

pub fn render(input: &str, ctx: &Context) -> Result<String> {
    let mut tera = Tera::default();
    let output = tera.render_str(input, &ctx.ctx)?;
    Ok(output)
}

static BASE_CONTEXT: LazyLock<tera::Context> = LazyLock::new(|| {
    let mut ctx = tera::Context::new();
    let root = git_util::find_work_tree_root();
    ctx.insert("color", &console::colors_enabled_stderr());
    ctx.insert("root", &root.display().to_string());
    ctx
});

#[derive(Clone)]
pub struct Context {
    ctx: tera::Context,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            ctx: BASE_CONTEXT.clone(),
        }
    }
}

impl Context {
    pub fn insert<T: Serialize + ?Sized, S: Into<String>>(&mut self, key: S, val: &T) {
        self.ctx.insert(key, val);
    }

    pub fn with_globs<P: AsRef<Path>>(&mut self, globs: &[P]) -> &mut Self {
        let globs = globs.iter().map(|m| m.as_ref().to_str().unwrap()).join(" ");
        self.insert("globs", &globs);
        self
    }

    pub fn with_files<P: AsRef<Path>>(&mut self, shell_type: ShellType, files: &[P]) -> &mut Self {
        let files_list: Vec<String> = files
            .iter()
            .map(|f| f.as_ref().to_str().unwrap().to_string())
            .collect();
        self.insert("files_list", &files_list);
        let quoted_files = files
            .iter()
            .map(|m| shell_type.quote(m.as_ref().to_str().unwrap()))
            .join(" ");
        self.insert("files", &quoted_files);
        self
    }

    pub fn with_workspace_indicator<P: AsRef<Path>>(
        &mut self,
        workspace_indicator: &P,
    ) -> &mut Self {
        let workspace_indicator = workspace_indicator.as_ref();
        let workspace_dir = workspace_indicator
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        self.insert("workspace", &workspace_dir.display().to_string());
        self.insert(
            "workspace_indicator",
            &workspace_indicator.display().to_string(),
        );
        self
    }

    pub fn with_workspace_files<P: AsRef<Path>>(
        &mut self,
        shell_type: ShellType,
        workspace_dir: &Path,
        files: &[P],
    ) -> &mut Self {
        let files = files
            .iter()
            .map(|m| {
                let p = m.as_ref();
                let rel = p.strip_prefix(workspace_dir).unwrap_or(p);
                shell_type.quote(rel.to_str().unwrap())
            })
            .join(" ");
        self.insert("workspace_files", &files);
        self
    }

    /// Returns a clone of this context where `files` and `workspace_files`
    /// are truncated to "first_file …" when there is more than one file.
    /// Used to render the human-readable progress message — keeps the
    /// rendered command compact for steps matching hundreds of files
    /// while still showing one concrete path.
    pub fn for_display(&self) -> Self {
        let mut ctx = self.clone();
        if let Some(truncated) = truncate_quoted_list(self.ctx.get("files")) {
            ctx.insert("files", &truncated);
        }
        if let Some(truncated) = truncate_quoted_list(self.ctx.get("workspace_files")) {
            ctx.insert("workspace_files", &truncated);
        }
        ctx
    }
}

/// Truncate a space-separated quoted-token list to "first …" when it
/// contains more than one token. Returns `None` if the value is missing,
/// not a string, or has 0–1 tokens (no truncation needed).
fn truncate_quoted_list(value: Option<&tera::Value>) -> Option<String> {
    let s = value.and_then(|v| v.as_str())?;
    let mut tokens = split_quoted_tokens(s);
    let first = tokens.next()?;
    if tokens.next().is_none() {
        return None;
    }
    Some(format!("{first} …"))
}

/// Split a `with_files`-style joined string back into its quoted tokens.
/// Tokens are space-separated; quoted tokens preserve embedded spaces.
/// Honors `\\` escapes inside quoted runs because that's what `ShellType::quote`
/// emits on Unix shells.
fn split_quoted_tokens(s: &str) -> impl Iterator<Item = &str> {
    let bytes = s.as_bytes();
    let mut start = 0usize;
    std::iter::from_fn(move || {
        while start < bytes.len() && bytes[start] == b' ' {
            start += 1;
        }
        if start >= bytes.len() {
            return None;
        }
        let token_start = start;
        let mut quote: Option<u8> = None;
        let mut i = start;
        while i < bytes.len() {
            let b = bytes[i];
            match quote {
                Some(q) => {
                    if b == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if b == q {
                        quote = None;
                    }
                }
                None => {
                    if b == b' ' {
                        break;
                    }
                    if b == b'\'' || b == b'"' {
                        quote = Some(b);
                    }
                }
            }
            i += 1;
        }
        let token = &s[token_start..i];
        start = i;
        Some(token)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn split(s: &str) -> Vec<&str> {
        split_quoted_tokens(s).collect()
    }

    #[test]
    fn split_quoted_tokens_handles_unquoted() {
        assert_eq!(split("a b c"), vec!["a", "b", "c"]);
    }

    #[test]
    fn split_quoted_tokens_preserves_single_quoted_spaces() {
        assert_eq!(
            split("'has space.txt' other.txt"),
            vec!["'has space.txt'", "other.txt"]
        );
    }

    #[test]
    fn split_quoted_tokens_handles_escaped_quote_in_double() {
        assert_eq!(split(r#""a\"b" c"#), vec![r#""a\"b""#, "c"]);
    }

    #[test]
    fn truncate_quoted_list_returns_none_for_single_token() {
        let v = json!("only.txt");
        assert_eq!(truncate_quoted_list(Some(&v)), None);
    }

    #[test]
    fn truncate_quoted_list_truncates_multiple_tokens() {
        let v = json!("first.txt second.txt third.txt");
        assert_eq!(
            truncate_quoted_list(Some(&v)),
            Some("first.txt …".to_string())
        );
    }

    #[test]
    fn truncate_quoted_list_preserves_quoted_first_token() {
        let v = json!("'a b.txt' other.txt");
        assert_eq!(
            truncate_quoted_list(Some(&v)),
            Some("'a b.txt' …".to_string())
        );
    }

    #[test]
    fn truncate_quoted_list_returns_none_for_empty_or_missing() {
        assert_eq!(truncate_quoted_list(None), None);
        let v = json!("");
        assert_eq!(truncate_quoted_list(Some(&v)), None);
        let v = json!("   ");
        assert_eq!(truncate_quoted_list(Some(&v)), None);
    }
}
