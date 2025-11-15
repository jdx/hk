use std::{path::Path, sync::LazyLock};

use crate::{Result, step::ShellType};
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
    let cwd = std::env::current_dir().expect("failed to get current directory");
    let root = xx::file::find_up(&cwd, &[".git"])
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or(cwd);
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
        let files = files
            .iter()
            .map(|m| shell_type.quote(m.as_ref().to_str().unwrap()))
            .join(" ");
        self.insert("files", &files);
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

    pub fn with_filepaths_file<P: AsRef<Path>>(
        &mut self,
        filepaths_file: &mut tempfile::NamedTempFile,
        files: &[P],
    ) -> crate::Result<&mut Self> {
        use std::io::Write;
        for f in files {
            writeln!(filepaths_file, "{}", f.as_ref().display())?;
        }
        self.insert(
            "filepaths_file",
            &filepaths_file.path().display().to_string(),
        );
        Ok(self)
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
}
