//! Working directory resolution for steps.
//!
//! `dir` is a Tera template, so a step can follow each job's workspace with
//! `dir = "{{workspace}}"` instead of opening its command with a `cd`. It is
//! read for two purposes:
//!
//! - Execution ([`Step::render_dir`]) - the working directory for the command
//!   and for `git apply`, rendered per job once the workspace is known
//! - Selection ([`Step::dir_prefix`]) - file filtering, glob prefixing, and
//!   stage pathspecs, which run before jobs exist and so can only use the
//!   literal prefix of `dir`

use crate::{Result, tera};
use indexmap::IndexSet;
use itertools::Itertools;
use std::path::PathBuf;

use super::Step;

/// Tera expression/statement openers. A `dir` containing either is a template.
const TEMPLATE_OPENERS: [&str; 2] = ["{{", "{%"];

impl Step {
    /// The literal directory prefix of `dir`, for selecting files before jobs
    /// (and therefore workspaces) exist.
    ///
    /// # Returns
    ///
    /// The whole value for a literal `dir`, otherwise the path prefix preceding
    /// the first template expression. Every directory the template can render
    /// to lives under that prefix, so filtering by it stays a superset of the
    /// eventual working directory. `None` when there is no such prefix or it
    /// names the repo root.
    pub fn dir_prefix(&self) -> Option<&str> {
        dir_prefix(self.dir.as_deref()?)
    }

    /// Render `dir` against a job's template context, resolving `{{workspace}}`
    /// to the workspace that job was built for.
    pub fn render_dir(&self, tctx: &tera::Context) -> Result<Option<String>> {
        let Some(dir) = self.dir.as_deref() else {
            return Ok(None);
        };
        let rendered = tera::render(dir, tctx)?;
        Ok(Some(strip_cur_dir(&rendered).to_string()))
    }

    /// Every directory `dir` resolved to across a step's jobs.
    ///
    /// Staging runs once per step rather than per job, so a templated `dir`
    /// has to be resolved again here: rendering it against each workspace the
    /// jobs were built from recovers the directories they ran in. Falls back
    /// to [`Step::dir_prefix`] when there are no workspaces to render with.
    ///
    /// # Returns
    ///
    /// An empty vector when `dir` is unset, meaning the repo root.
    pub fn resolved_dirs(
        &self,
        base: &tera::Context,
        files: &IndexSet<PathBuf>,
    ) -> Result<Vec<String>> {
        let Some(dir) = self.dir.as_deref() else {
            return Ok(Vec::new());
        };
        if template_start(dir).is_none() {
            return Ok(vec![strip_cur_dir(dir).to_string()]);
        }
        let workspaces = self
            .workspaces_for_files(&files.iter().cloned().collect_vec())?
            .unwrap_or_default();
        if workspaces.is_empty() {
            return Ok(self
                .dir_prefix()
                .map(ToString::to_string)
                .into_iter()
                .collect());
        }
        render_per_workspace(self, base, &workspaces)
    }
}

/// Render `dir` once per workspace, as each job did.
fn render_per_workspace(
    step: &Step,
    base: &tera::Context,
    workspaces: &IndexSet<PathBuf>,
) -> Result<Vec<String>> {
    let mut dirs = Vec::new();
    for workspace_indicator in workspaces {
        let mut tctx = base.clone();
        tctx.insert("step", &step.name);
        tctx.with_workspace_indicator(workspace_indicator);
        let dir = step.render_dir(&tctx)?.unwrap_or_default();
        if !dir.is_empty() && !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    Ok(dirs)
}

/// The literal path prefix of `dir`: everything before the separator preceding
/// the first template expression.
fn dir_prefix(dir: &str) -> Option<&str> {
    let dir = strip_cur_dir(dir);
    let Some(start) = template_start(dir) else {
        return subdir_prefix(dir);
    };
    // Only whole segments count: "pkg-" in `pkg-{{workspace}}` is not a directory.
    let (prefix, _) = dir[..start].rsplit_once(['/', '\\'])?;
    subdir_prefix(prefix.trim_end_matches(['/', '\\']))
}

/// Byte offset of the first template expression, or `None` for a literal `dir`.
fn template_start(dir: &str) -> Option<usize> {
    TEMPLATE_OPENERS
        .iter()
        .filter_map(|opener| dir.find(opener))
        .min()
}

/// Discard a prefix that names the repo root rather than a subdirectory, which
/// narrows nothing and would corrupt a stage pathspec built from it.
fn subdir_prefix(prefix: &str) -> Option<&str> {
    (!prefix.is_empty() && prefix != ".").then_some(prefix)
}

/// Strip leading `./` (and `.\`) segments.
///
/// `Path::components` keeps a leading `.` as `Component::CurDir`, so
/// `Path::new("sub/a.go").starts_with("./sub")` is false and `strip_prefix`
/// fails where the bare `sub` form succeeds. Without this, `dir = "./sub"`
/// matches no files at all and `{{files}}` keeps a prefix the command has
/// already changed into. Non-leading `.` needs no handling: `Path` normalizes
/// it away.
fn strip_cur_dir(dir: &str) -> &str {
    let mut rest = dir;
    while let Some(stripped) = rest.strip_prefix("./").or_else(|| rest.strip_prefix(".\\")) {
        // `./` alone is the repo root, not an empty path.
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
        // A subproject prefixes `dir` with its own directory; that must keep
        // scoping file selection.
        assert_eq!(dir_prefix("sub/{{workspace}}"), Some("sub"));
        assert_eq!(dir_prefix("a/b/{{workspace}}/c"), Some("a/b"));
        assert_eq!(dir_prefix("sub\\{{workspace}}"), Some("sub"));
    }

    #[test]
    fn partial_segment_before_template_is_not_a_directory() {
        assert_eq!(dir_prefix("pkg-{{workspace}}"), None);
    }

    #[test]
    fn leading_cur_dir_is_normalized_away() {
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
