use crate::Result;
use std::path::PathBuf;

use crate::{lsp_types::{CodeAction, Diagnostic, Range, Position, Severity, WorkspaceEdit, TextEdit, CodeActionKind}, plugins::plugin::Plugin};

#[derive(Debug, Default)]
pub struct EndOfFileFixer {}

impl Plugin for EndOfFileFixer {
    fn name(&self) -> &'static str {
        "end-of-file-fixer"
    }

    fn lint(&self, files: &[PathBuf]) -> Result<(Vec<Diagnostic>, Vec<CodeAction>)> {
        let mut diagnostics = Vec::new();
        let mut actions = Vec::new();

        for file in files {
            let contents = std::fs::read_to_string(file)?;
            if !contents.ends_with('\n') {
                // Create diagnostic for missing newline
                let last_line = contents.lines().count().saturating_sub(1) as u32;
                let last_char = if let Some(last_line) = contents.lines().last() {
                    last_line.len() as u32
                } else {
                    0
                };
                
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position { line: last_line, character: 0 },
                        end: Position { line: last_line, character: last_char },
                    },
                    severity: Some(Severity::Warning),
                    code: Some("end-of-file-fixer".to_string()),
                    code_description: None,
                    source: Some("pre-commit".to_string()),
                    message: "File does not end with a newline".to_string(),
                    tags: vec![],
                    related_information: vec![],
                };
                
                // Create code action to fix the issue
                let edit = WorkspaceEdit {
                    changes: [(
                        file.to_string_lossy().to_string(),
                        vec![TextEdit {
                            range: diagnostic.range.clone(),
                            new_text: format!("{}\n", contents),
                        }],
                    )].into_iter().collect(),
                };

                let action = CodeAction {
                    title: "Add newline at end of file".to_string(),
                    kind: Some(CodeActionKind::QuickFix),
                    diagnostics: vec![diagnostic.clone()],
                    is_preferred: true,
                    disabled: None,
                    edit: Some(edit),
                    command: None,
                };

                diagnostics.push(diagnostic);
                actions.push(action);
            }
        }

        Ok((diagnostics, actions))
    }
}
