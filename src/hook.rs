use serde::Deserialize;
use std::path::PathBuf;

use serde_with::{serde_as, OneOrMany};

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
#[serde(deny_unknown_fields)]
pub struct Hook {
    pub name: String,
    pub plugin: Option<String>,
    // pub run: Option<String>,
    pub list_files_with_errors: Option<String>,
    pub fix: Option<String>,
    pub render_error_json: Option<String>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub glob: Option<Vec<String>>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub exclude: Option<Vec<String>>,
    #[serde_as(as = "Option<OneOrMany<_>>")]
    pub file_types: Option<Vec<FileType>>,
    pub root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileType {
    Text,
    Binary,
    Executable,
    NotExecutable,
    Symlink,
    NotSymlink,
}

// impl Hook {
//     pub fn name(&self) -> &str {
//         self.name.as_deref().unwrap_or(
//             self.plugin.as_deref().unwrap_or(
//                 self.list_files_with_errors
//                     .as_deref()
//                     .and_then(|r| r.split_whitespace().next())
//                     .unwrap_or("angler"),
//             ),
//         )
//     }
// }
