use crate::lsp_types::{CodeAction, Diagnostic};
use crate::Result;
use std::path::PathBuf;

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn lint(&self, files: &[PathBuf]) -> Result<(Vec<Diagnostic>, Vec<CodeAction>)>;
}
