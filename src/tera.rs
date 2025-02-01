use std::{
    path::Path,
    sync::LazyLock,
};

use crate::Result;
use itertools::Itertools;
use serde::Serialize;
use tera::{Tera, Value};

pub fn render(input: &str, ctx: &Context) -> Result<String> {
    let mut tera = Tera::default();
    let output = tera.render_str(input, &ctx.ctx)?;
    Ok(output)
}

static BASE_CONTEXT: LazyLock<tera::Context> = LazyLock::new(tera::Context::new);

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

    pub fn with_staged_files<P: AsRef<Path>>(&mut self, files: &[P]) -> &mut Self {
        let staged_files = files
            .iter()
            .map(|m| {
                let s = m.as_ref().to_str().unwrap();
                if s.contains(" ") {
                    format!("'{s}'")
                } else {
                    s.to_string()
                }
            })
            .join(" ");
        self.ctx.insert("staged_files", &staged_files);
        self
    }

    pub fn with_files<P: AsRef<Path>>(&mut self, files: &[P]) -> &mut Self {
        let files = files
            .iter()
            .map(|m| {
                let s = m.as_ref().to_str().unwrap();
                if s.contains(" ") {
                    format!("'{s}'")
                } else {
                    s.to_string()
                }
            })
            .join(" ");
        self.ctx.insert("files", &files);
        self
    }
}
