//! Resolution of a step's `dir`.
//!
//! `dir` is a Tera template, so a step can point its working directory at a
//! value that is only known per job — most usefully `{{workspace}}`:
//!
//! ```ignore
//! ["go-vet"] {
//!     glob = "**/*.go"
//!     workspace_indicator = "go.mod"
//!     dir = "{{workspace}}"
//!     check = "go vet ./..."
//! }
//! ```
//!
//! That template can only be rendered once a job exists, because the workspace
//! is picked per job. `dir` is read at two kinds of site:
//!
//! - **Execution sites** ([`Step::render_dir`]) — the child process cwd and the
//!   cwd for `git apply`. These run per job, so they render the template.
//! - **Selection sites** ([`Step::dir_prefix`]) — file filtering, glob
//!   prefixing, and stage pathspecs. These run before jobs exist, so they fall
//!   back to the literal path prefix that precedes the first template
//!   expression: `sub/{{workspace}}` scopes to `sub`, and a `dir` that is
//!   entirely a template scopes to nothing.
//!
//! Both accessors strip a leading `./`, which `Path::components` would
//! otherwise keep as a `CurDir` component that matches no repo-relative file
//! path. Literal `dir` values contain no template expression, so apart from
//! that normalization both accessors return the input unchanged.

use crate::{Result, tera};

use super::types::Step;

/// Tera expression/statement openers. A `dir` containing either is a template.
const TEMPLATE_OPENERS: [&str; 2] = ["{{", "{%"];

impl Step {
    /// The literal directory prefix of `dir`, used to select files before jobs
    /// (and therefore workspaces) exist.
    ///
    /// Returns the whole value for a literal `dir`. For a templated `dir`, this
    /// is the path prefix preceding the first template expression, which every
    /// value the template can render to lives under — so filtering by it stays
    /// a superset of the eventual working directory. Returns `None` when the
    /// template starts in the first path segment and there is no such prefix,
    /// and when the prefix is the repo root itself.
    pub fn dir_prefix(&self) -> Option<&str> {
        dir_prefix(self.dir.as_deref()?)
    }

    /// The step's working directory for this job, with `dir` rendered against
    /// the job's template context (so `{{workspace}}` resolves to the
    /// workspace this job was built for).
    pub fn render_dir(&self, tctx: &tera::Context) -> Result<Option<String>> {
        let Some(dir) = self.dir.as_deref() else {
            return Ok(None);
        };
        let rendered = tera::render(dir, tctx)?;
        Ok(Some(strip_cur_dir(&rendered).to_string()))
    }
}

/// The literal path prefix of `dir` — everything before the path separator
/// that precedes the first template expression.
fn dir_prefix(dir: &str) -> Option<&str> {
    let dir = strip_cur_dir(dir);
    let Some(template_start) = TEMPLATE_OPENERS
        .iter()
        .filter_map(|opener| dir.find(opener))
        .min()
    else {
        return keep_narrowing_prefix(dir);
    };
    // Only whole path segments count: in `pkg-{{workspace}}` the text before
    // the expression is not a directory of its own.
    let head = &dir[..template_start];
    keep_narrowing_prefix(head[..head.rfind(['/', '\\'])?].trim_end_matches(['/', '\\']))
}

/// Drop prefixes that name the repo root rather than a subdirectory — there is
/// nothing to narrow file selection to, and prefixing a stage glob with them
/// would corrupt the pathspec.
fn keep_narrowing_prefix(prefix: &str) -> Option<&str> {
    (!prefix.is_empty() && prefix != ".").then_some(prefix)
}

/// Strip leading `./` (and `.\`) segments from a directory.
///
/// `Path::components` preserves a leading `.` as `Component::CurDir`, so
/// `Path::new("sub/a.go").starts_with("./sub")` is false and
/// `strip_prefix("./sub")` fails where the bare `sub` form succeeds. Without
/// this, `dir = "./sub"` selects no files at all, and a `dir` that renders to
/// `./sub` leaves `{{files}}` repo-root-relative while the command runs in
/// `sub`, so every path is doubled. Both separators are handled on every
/// platform, matching how [`dir_prefix`] splits segments.
fn strip_cur_dir(dir: &str) -> &str {
    let mut rest = dir;
    while let Some(stripped) = rest.strip_prefix("./").or_else(|| rest.strip_prefix(".\\")) {
        // `./` on its own is the repo root, not an empty path.
        if stripped.is_empty() {
            return ".";
        }
        rest = stripped;
    }
    rest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_dir_is_its_own_prefix() {
        assert_eq!(dir_prefix("packages/web"), Some("packages/web"));
        assert_eq!(dir_prefix("frontend"), Some("frontend"));
    }

    #[test]
    fn fully_templated_dir_has_no_prefix() {
        assert_eq!(dir_prefix("{{workspace}}"), None);
        assert_eq!(dir_prefix("./{{workspace}}"), None);
        assert_eq!(dir_prefix("{% if x %}a{% endif %}"), None);
    }

    #[test]
    fn templated_dir_keeps_its_literal_prefix() {
        // A subproject scopes its steps by prefixing `dir` with the subproject
        // directory; that prefix must keep scoping file selection.
        assert_eq!(dir_prefix("sub/{{workspace}}"), Some("sub"));
        assert_eq!(dir_prefix("a/b/{{workspace}}/c"), Some("a/b"));
        assert_eq!(dir_prefix("sub\\{{workspace}}"), Some("sub"));
    }

    #[test]
    fn partial_segment_before_template_is_not_a_directory() {
        // `pkg-{{workspace}}` has no complete literal segment: "pkg-" is not a
        // directory, so selection must not be narrowed to it.
        assert_eq!(dir_prefix("pkg-{{workspace}}"), None);
    }

    #[test]
    fn leading_cur_dir_is_normalized_away() {
        // `Path::starts_with` keeps a leading "." as a CurDir component, so an
        // un-normalized "./sub" would match no repo-relative path and select
        // nothing at all.
        assert_eq!(dir_prefix("./sub"), Some("sub"));
        assert_eq!(dir_prefix(".\\sub"), Some("sub"));
        assert_eq!(dir_prefix("./sub/{{workspace}}"), Some("sub"));
        assert_eq!(dir_prefix(".\\sub\\{{workspace}}"), Some("sub"));
        assert_eq!(dir_prefix("././sub"), Some("sub"));
    }

    #[test]
    fn repo_root_dir_narrows_nothing() {
        assert_eq!(dir_prefix("."), None);
        assert_eq!(dir_prefix("./"), None);
        assert_eq!(dir_prefix(""), None);
    }

    #[test]
    fn strip_cur_dir_leaves_other_paths_alone() {
        assert_eq!(strip_cur_dir("sub/api"), "sub/api");
        assert_eq!(strip_cur_dir("."), ".");
        assert_eq!(strip_cur_dir(".hidden/api"), ".hidden/api");
        assert_eq!(strip_cur_dir("sub/./api"), "sub/./api");
    }

    #[test]
    fn render_dir_normalizes_the_rendered_value() {
        let mut tctx = tera::Context::default();
        tctx.insert("workspace", "sub/api");
        let step = Step {
            dir: Some("./{{workspace}}".to_string()),
            ..Default::default()
        };
        assert_eq!(step.render_dir(&tctx).unwrap().as_deref(), Some("sub/api"));
    }

    #[test]
    fn render_dir_keeps_a_workspace_at_the_repo_root() {
        // `{{workspace}}` is "." for an indicator at the repo root: a valid cwd
        // that must not collapse to an empty path.
        let mut tctx = tera::Context::default();
        tctx.insert("workspace", ".");
        let step = Step {
            dir: Some("{{workspace}}".to_string()),
            ..Default::default()
        };
        assert_eq!(step.render_dir(&tctx).unwrap().as_deref(), Some("."));
    }
}
