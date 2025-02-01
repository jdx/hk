#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Plugin {
    pub name: String,
    pub format: PluginFormat,
    pub meta: PluginMeta,
    pub file_types: Vec<FileType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginFormat {
    pub command: String,
    pub args: Vec<String>,
    pub to_stdin: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginMeta {
    pub description: String,
    pub url: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, strum::EnumString)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    JavaScript,
    JavaScriptReact,
    TypeScript,
    TypeScriptReact,
    Vue,
    CSS,
    SCSS,
    Less,
    HTML,
    JSON,
    JSONC,
    YAML,
    Markdown,
    #[serde(rename = "markdown.mdx")]
    MarkdownMDX,
    GraphQL,
    Handlebars,
    Svelte,
    Astro,
    HTMLAngular,
}
