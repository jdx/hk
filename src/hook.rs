use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use serde_with::{serde_as, OneOrMany};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct FileLocks {
    pub read: Option<String>,
    pub write: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct Hook {
    pub r#type: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub glob: Option<Vec<String>>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub file_types: Option<Vec<String>>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub file_kind: Option<Vec<FileKind>>,
    pub run: Option<String>,
    pub root: Option<PathBuf>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub single_file: Option<bool>,
    pub exclusive: Option<bool>,
    pub to_stdin: Option<bool>,
    pub to_temp_file: Option<bool>,
    pub from_stderr: Option<bool>,
    pub depends: Option<Vec<String>>,
    pub file_locks: Option<FileLocks>,
    pub stage: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Text,
    Binary,
    Executable,
    NotExecutable,
    Symlink,
    NotSymlink,
}
