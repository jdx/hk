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
//! Literal `dir` values contain no template expression, so both accessors
//! return the same string and behavior is unchanged.

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
    /// template starts in the first path segment and there is no such prefix.
    pub fn dir_prefix(&self) -> Option<&str> {
        dir_prefix(self.dir.as_deref()?)
    }

    /// The step's working directory for this job, with `dir` rendered against
    /// the job's template context (so `{{workspace}}` resolves to the
    /// workspace this job was built for).
    pub fn render_dir(&self, tctx: &tera::Context) -> Result<Option<String>> {
        self.dir
            .as_deref()
            .map(|dir| tera::render(dir, tctx))
            .transpose()
    }
}

/// The literal path prefix of `dir` — everything before the path separator
/// that precedes the first template expression.
fn dir_prefix(dir: &str) -> Option<&str> {
    let Some(template_start) = TEMPLATE_OPENERS
        .iter()
        .filter_map(|opener| dir.find(opener))
        .min()
    else {
        return Some(dir);
    };
    // Only whole path segments count: in `pkg-{{workspace}}` the text before
    // the expression is not a directory of its own.
    let head = &dir[..template_start];
    let prefix = head[..head.rfind(['/', '\\'])?].trim_end_matches(['/', '\\']);
    // `{{workspace}}` and `./{{workspace}}` are both rooted at the repo root:
    // there is no literal directory to narrow file selection to.
    (!prefix.is_empty() && prefix != ".").then_some(prefix)
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
}
