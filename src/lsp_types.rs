use indexmap::IndexMap;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<Severity>,
    pub code: Option<String>,
    pub code_description: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub tags: Vec<DiagnosticTag>,
    pub related_information: Vec<DiagnosticRelatedInformation>,
    // data: Option<LSPAny>,
}

#[derive(Debug, Clone)]
pub enum Severity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone)]
pub enum DiagnosticTag {
    Unnecessary,
    Deprecated,
}

#[derive(Debug, Clone)]
pub struct DiagnosticRelatedInformation {
    pub location: Location,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone)]
pub struct CodeAction {
    pub title: String,
    pub kind: Option<CodeActionKind>,
    pub diagnostics: Vec<Diagnostic>,
    pub is_preferred: bool,
    pub disabled: Option<CodeActionDisabled>,
    pub edit: Option<WorkspaceEdit>,
    pub command: Option<Command>,
    // data: Option<LSPAny>,
}

#[derive(Debug, Clone)]
pub enum CodeActionKind {
    QuickFix,
    SourceFix,
    SourceFixAll,
    SourceOrganizeImports,
}

#[derive(Debug, Clone)]
pub enum CodeActionDisabled {
    Reason(String),
}

#[derive(Debug, Clone)]
pub struct WorkspaceEdit {
    pub changes: IndexMap<String, Vec<TextEdit>>,
}

#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub title: String,
    pub command: String,
    // arguments: Vec<LSPAny>,
}

impl Diagnostic {
    pub fn to_string(&self) -> String {
        format!(
            "{}:{}: {}: {}",
            self.range.start.line,
            self.range.start.character,
            self.code.as_ref().unwrap_or(&"".to_string()),
            self.message
        )
    }
}
