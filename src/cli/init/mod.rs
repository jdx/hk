mod detector;
mod generator;
mod picker;

use std::io::IsTerminal;
use std::path::PathBuf;

use crate::builtins::BuiltinMeta;
use crate::{Result, env};
use eyre::eyre;
use toml_edit::{DocumentMut, Item, Table, TableLike, value};

/// Default hooks to configure when none are specified
pub(crate) const DEFAULT_HOOKS: &[&str] = &["pre-commit"];

/// Generate a new hk.pkl file for a project
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "write")]
pub struct Init {
    /// Overwrite existing hk.pkl file
    #[usage(short, long)]
    force: bool,
    /// Interactive mode: select linters and hooks manually
    #[usage(short, long)]
    interactive: bool,
    /// Generate a mise.toml file with hk configured
    ///
    /// Also adds the mise tools that the detected or selected linters run,
    /// such as `ruff` or `npm:eslint`, at `latest`. Tools already in
    /// mise.toml are kept, and language runtimes such as `rust` or `go` are
    /// left out.
    ///
    /// Set HK_MISE=1 to make this the default behavior.
    #[usage(long, verbatim_doc_comment)]
    mise: bool,
}

impl Init {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.pkl");

        // An existing hk.pkl makes a repeat init harmless: the picker never
        // opens, so that case wins over the terminal check below.
        let skip_existing = hk_file.exists() && !self.force;

        // The picker needs a terminal; fail before writing anything.
        if self.interactive
            && !skip_existing
            && !(std::io::stdin().is_terminal() && std::io::stderr().is_terminal())
        {
            return Err(eyre!(
                "`hk init --interactive` needs a terminal to prompt for linters; run it in a terminal or drop `-i` to auto-detect"
            ));
        }

        let mise = *env::HK_MISE || self.mise;

        // An existing hk.pkl is left alone, so there are no newly chosen
        // linters whose tools to add; mise.toml only gets hk.
        if skip_existing {
            if mise {
                self.write_mise_toml(&[])?;
            }
            warn!("hk.pkl already exists, run with --force to overwrite");
            return Ok(());
        }

        // Fail on an unusable mise.toml before the picker asks anything.
        if mise {
            self.plan_mise_toml(&[])?;
        }

        // Detect project files
        let project_root = std::env::current_dir()?;
        let detections = detector::detect_builtins(&project_root);

        let (hook_content, builtins) = if self.interactive {
            // Interactive mode: let user pick from all builtins
            self.run_interactive(&detections)?
        } else {
            // Auto mode (default): use detected builtins or fall back to template
            let builtins: Vec<_> = detections.iter().map(|d| d.builtin).collect();
            (self.run_auto(&builtins), builtins)
        };
        let has_steps = !builtins.is_empty();

        // Write mise.toml before hk.pkl so a mise.toml error leaves both untouched.
        if mise {
            self.write_mise_toml(&builtins)?;
        }

        // Write the file
        xx::file::write(&hk_file, &hook_content)?;

        // Print summary
        if !detections.is_empty() && !self.interactive {
            let summary = detections
                .iter()
                .map(|d| format!("{} ({})", d.builtin.name, d.reason))
                .collect::<Vec<_>>()
                .join(", ");
            info!("Detected: {}", summary);
        }
        info!("Created hk.pkl");
        if !has_steps {
            info!("No linters selected or detected, so hk.pkl has no steps yet.");
            info!(
                "Next: add steps to hk.pkl (see https://hk.jdx.dev/configuration), then run `hk check --all`"
            );
        } else {
            info!("Next: run `hk check --all` to try it, then `hk install` to set up hooks");
        }

        Ok(())
    }

    fn run_interactive(
        &self,
        detections: &[detector::Detection],
    ) -> Result<(String, Vec<&'static BuiltinMeta>)> {
        // Print detection info
        if !detections.is_empty() {
            info!("Scanning project...");
            for detection in detections {
                info!(
                    "  Detected: {} ({})",
                    detection.builtin.name, detection.reason
                );
            }
        }

        // Let user pick builtins
        let builtins = picker::pick_builtins(detections)?;

        if builtins.is_empty() {
            return Ok((
                generator::generate_default_template(&crate::pkl_modules::import_root()),
                builtins,
            ));
        }

        // Let user pick hooks
        let hooks = picker::pick_hooks()?;

        let hooks = if hooks.is_empty() {
            warn!("No hooks selected, using defaults");
            DEFAULT_HOOKS.iter().map(|s| s.to_string()).collect()
        } else {
            hooks
        };

        let content =
            generator::generate_pkl(&builtins, &hooks, &crate::pkl_modules::import_root());
        Ok((content, builtins))
    }

    fn run_auto(&self, builtins: &[&'static BuiltinMeta]) -> String {
        if builtins.is_empty() {
            // No detections - use default template
            return generator::generate_default_template(&crate::pkl_modules::import_root());
        }

        // Use detected builtins with default hooks
        let hooks: Vec<String> = DEFAULT_HOOKS.iter().map(|s| s.to_string()).collect();

        generator::generate_pkl(builtins, &hooks, &crate::pkl_modules::import_root())
    }

    /// Read mise.toml and merge hk's entries and the tools `builtins` run.
    fn plan_mise_toml(&self, builtins: &[&'static BuiltinMeta]) -> Result<MisePlan> {
        let mise_file = PathBuf::from("mise.toml");
        let original = mise_file
            .exists()
            .then(|| std::fs::read_to_string(&mise_file))
            .transpose()?;
        let root = std::env::current_dir()?;
        let input = original.as_deref().unwrap_or("");
        let suppress_pre_commit = match included_pre_commit(&root, input)? {
            Some(value) => value,
            None => existing_pre_commit_file_task(&root),
        };
        let merged = merge_mise_config(input, suppress_pre_commit, &builtin_tools(builtins))?;
        Ok(MisePlan { original, merged })
    }

    fn write_mise_toml(&self, builtins: &[&'static BuiltinMeta]) -> Result<()> {
        let MisePlan { original, merged } = self.plan_mise_toml(builtins)?;
        if merged.external_pre_commit {
            warn!(
                "Preserving external mise task configuration for pre-commit; wire hk manually if needed"
            );
        }
        if original.as_deref() != Some(merged.content.as_str()) {
            xx::file::write("mise.toml", &merged.content)?;
            if original.is_none() {
                info!("Generated mise.toml");
            } else {
                info!("Updated mise.toml");
            }
        } else {
            info!("mise.toml already contains hk configuration");
        }
        if !merged.added_tools.is_empty() {
            info!(
                "Added linter tools to mise.toml: {}; run `mise install` to install them",
                merged.added_tools.join(", ")
            );
        }
        if !merged.skipped_runtimes.is_empty() {
            info!(
                "Left language runtimes out of mise.toml: {}; add them with `mise use {}` if mise should manage them",
                merged.skipped_runtimes.join(", "),
                merged.skipped_runtimes.join(" ")
            );
        }
        Ok(())
    }
}

struct MisePlan {
    original: Option<String>,
    merged: MiseMerge,
}

/// Language runtimes that some builtins run. `hk init --mise` doesn't pin
/// them at `latest`: a project usually picks their version itself (for
/// example in `rust-toolchain.toml` or `go.mod`), and a mise pin would
/// override it.
const RUNTIME_TOOLS: &[&str] = &[
    "bun", "deno", "dotnet", "elixir", "erlang", "go", "java", "node", "python", "ruby", "rust",
    "swift", "zig",
];

/// The mise tools that `builtins` run, in order, without duplicates.
fn builtin_tools(builtins: &[&'static BuiltinMeta]) -> Vec<&'static str> {
    let mut tools: Vec<&'static str> = Vec::new();
    for tool in builtins.iter().flat_map(|builtin| builtin.mise_tools) {
        if !tools.contains(tool) {
            tools.push(tool);
        }
    }
    tools
}

/// Detect conventional mise file-task locations for pre-commit.
fn existing_pre_commit_file_task(root: &std::path::Path) -> bool {
    [
        "mise-tasks",
        ".mise-tasks",
        "mise/tasks",
        ".mise/tasks",
        ".config/mise/tasks",
    ]
    .iter()
    .any(|dir| {
        let base = root.join(dir).join("pre-commit");
        base.is_file() || base.join("_default").is_file()
    })
}

/// Return whether explicit task includes should suppress root task insertion.
/// `None` means no includes were configured, so conventional defaults apply.
fn included_pre_commit(root: &std::path::Path, input: &str) -> Result<Option<bool>> {
    let document = if input.is_empty() {
        DocumentMut::new()
    } else {
        input
            .parse::<DocumentMut>()
            .map_err(|error| eyre!("invalid mise.toml: {error}"))?
    };
    let Some(task_config) = document.get("task_config") else {
        return Ok(None);
    };
    let task_config = task_config
        .as_table_like()
        .ok_or_else(|| eyre!("unsupported mise.toml: [task_config] must be a table"))?;
    let Some(includes) = task_config.get("includes") else {
        return Ok(None);
    };
    let Some(includes) = includes.as_array() else {
        warn!("Unable to inspect mise task includes; preserving external task configuration");
        return Ok(Some(true));
    };
    for include in includes {
        let Some(path) = include.as_str() else {
            warn!("Unable to inspect mise task includes; preserving external task configuration");
            return Ok(Some(true));
        };
        if path.contains("://") || path.contains(['$', '{', '}', '*', '?', '[', ']', '~']) {
            warn!("Unable to inspect mise task includes; preserving external task configuration");
            return Ok(Some(true));
        }
        let path = root.join(path);
        if path.is_dir() {
            let task = path.join("pre-commit");
            for candidate in [task.clone(), task.join("_default")] {
                match std::fs::metadata(candidate) {
                    Ok(metadata) if metadata.is_file() => return Ok(Some(true)),
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => {
                        warn!(
                            "Unable to inspect mise task includes; preserving external task configuration"
                        );
                        return Ok(Some(true));
                    }
                }
            }
            match scan_included_task_dir(&path) {
                Ok(true) => return Ok(Some(true)),
                Ok(false) => {}
                Err(_) => {
                    warn!(
                        "Unable to inspect mise task includes; preserving external task configuration"
                    );
                    return Ok(Some(true));
                }
            }
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => {
                warn!(
                    "Unable to inspect mise task includes; preserving external task configuration"
                );
                return Ok(Some(true));
            }
        };
        let document = match content.parse::<DocumentMut>() {
            Ok(document) => document,
            Err(_) => {
                warn!(
                    "Unable to inspect mise task includes; preserving external task configuration"
                );
                return Ok(Some(true));
            }
        };
        if document.get("pre-commit").is_some() {
            return Ok(Some(true));
        }
    }
    Ok(Some(false))
}

fn scan_included_task_dir(path: &std::path::Path) -> std::io::Result<bool> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(std::io::Error::other("symlinked mise task include"));
        }
        if metadata.is_dir() {
            if scan_included_task_dir(&path)? {
                return Ok(true);
            }
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
            let filename = name.to_string_lossy();
            if filename == "mise.toml"
                || (filename.starts_with("mise.") && filename.ends_with(".toml"))
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "mise config in task include",
                ));
            }
            let content = std::fs::read_to_string(&path)?;
            let document = content.parse::<DocumentMut>().map_err(|error| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string())
            })?;
            if document.get("pre-commit").is_some() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// The result of merging hk's entries into a mise.toml.
#[derive(Debug)]
struct MiseMerge {
    content: String,
    /// Linter tools added to `[tools]`.
    added_tools: Vec<String>,
    /// Language runtimes the linters run that were not added.
    skipped_runtimes: Vec<String>,
    /// No pre-commit task was added because one is configured outside mise.toml.
    external_pre_commit: bool,
}

/// Merge hk's minimal mise entries, and the linter `tools` that are not
/// configured yet, without replacing user configuration.
fn merge_mise_config(
    input: &str,
    external_pre_commit: bool,
    tools_to_add: &[&str],
) -> Result<MiseMerge> {
    let mut document = if input.is_empty() {
        DocumentMut::new()
    } else {
        input
            .parse::<DocumentMut>()
            .map_err(|error| eyre!("invalid mise.toml: {error}"))?
    };
    if document.get("tools").is_none() {
        document["tools"] = Item::Table(Table::new());
    }
    let tools = document["tools"]
        .as_table_like_mut()
        .ok_or_else(|| eyre!("unsupported mise.toml: [tools] must be a table"))?;
    let has_hk = has_tool(tools, &["hk", "jdx/hk"]);
    if !has_hk {
        tools.insert("hk", value("latest"));
    }
    let mut added_tools = Vec::new();
    let mut skipped_runtimes = Vec::new();
    for &tool in tools_to_add {
        if has_mise_tool(tools, tool) {
            continue;
        }
        if RUNTIME_TOOLS.contains(&tool) {
            skipped_runtimes.push(tool.to_string());
            continue;
        }
        tools.insert(tool, value("latest"));
        added_tools.push(tool.to_string());
    }
    let has_pre_commit = match document.get("tasks") {
        Some(item) => item
            .as_table_like()
            .ok_or_else(|| eyre!("unsupported mise.toml: [tasks] must be a table"))?
            .get("pre-commit")
            .is_some(),
        None => false,
    };
    let mut reported_external_pre_commit = false;
    if !has_pre_commit {
        if external_pre_commit {
            reported_external_pre_commit = true;
        } else {
            if document.get("tasks").is_none() {
                let mut tasks = Table::new();
                tasks.set_implicit(true);
                document["tasks"] = Item::Table(tasks);
            }
            let tasks = document["tasks"]
                .as_table_like_mut()
                .expect("tasks was validated or created as a table");
            tasks.insert(
                "pre-commit",
                Item::Table(Table::from_iter([("run", value("hk run pre-commit"))])),
            );
        }
    }
    Ok(MiseMerge {
        content: document.to_string(),
        added_tools,
        skipped_runtimes,
        external_pre_commit: reported_external_pre_commit,
    })
}

/// Whether `[tools]` already configures `tool`, a `mise use` argument such as
/// `ruff`, `npm:eslint`, or `github:owner/repo[asset_pattern=*.zip]`.
///
/// A key and the tool match when they are the same spec, or when one is a
/// bare registry name and the other a backend spec for a package of that
/// name: `ruff` matches `aqua:astral-sh/ruff`, and `npm:eslint` matches
/// `eslint`.
fn has_mise_tool(tools: &dyn TableLike, tool: &str) -> bool {
    fn strip_options(spec: &str) -> &str {
        spec.split_once('[').map_or(spec, |(spec, _)| spec)
    }
    fn package_name(spec: &str) -> Option<&str> {
        let (_, package) = spec.split_once(':')?;
        package.rsplit('/').next()
    }
    let tool = strip_options(tool);
    tools.iter().any(|(key, _)| {
        let key = strip_options(key);
        key == tool
            || (!tool.contains(':') && package_name(key) == Some(tool))
            || (!key.contains(':') && package_name(tool) == Some(key))
    })
}

fn has_tool(tools: &dyn TableLike, suffixes: &[&str]) -> bool {
    tools.iter().any(|(key, _)| {
        suffixes.iter().any(|suffix| {
            key == *suffix || key.split_once(':').is_some_and(|(_, tool)| tool == *suffix)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_has_stable_output() {
        assert_eq!(
            merge_mise_config("", false, &[]).unwrap().content,
            "[tools]\nhk = \"latest\"\n\n[tasks.pre-commit]\nrun = \"hk run pre-commit\"\n"
        );
    }

    #[test]
    fn dotted_tools_and_implicit_tasks_have_stable_output() {
        assert_eq!(
            merge_mise_config("tools.node = \"20\"\n", false, &[])
                .unwrap()
                .content,
            "tools.node = \"20\"\ntools.hk = \"latest\"\n\n[tasks.pre-commit]\nrun = \"hk run pre-commit\"\n"
        );
    }

    #[test]
    fn inline_tables_have_stable_output() {
        assert_eq!(
            merge_mise_config(
                "tools = { node = \"20\" }\ntasks = { check = \"custom\" }\n",
                false,
                &[],
            )
            .unwrap()
            .content,
            "tools = { node = \"20\" , hk = \"latest\" }\ntasks = { check = \"custom\" , pre-commit = { run = \"hk run pre-commit\" } }\n"
        );
    }

    #[test]
    fn scalar_tasks_are_rejected() {
        assert!(merge_mise_config("tasks = \"custom\"\n", false, &[]).is_err());
    }

    #[test]
    fn backend_qualified_hk_is_recognized() {
        let output = merge_mise_config(
            "[tools]\n\"asdf:hk\" = \"1\"\n\"vfox:jdx/hk\" = \"2\"\n\"asdf:other\" = \"3\"\n",
            false,
            &[],
        )
        .unwrap()
        .content;
        assert!(output.contains("\"asdf:hk\" = \"1\""));
        assert!(output.contains("\"vfox:jdx/hk\" = \"2\""));
        assert!(!output.contains("\nhk = \"latest\""));
    }

    #[test]
    fn linter_tools_are_added_after_hk() {
        let merged = merge_mise_config("", false, &["ruff", "npm:prettier"]).unwrap();
        assert_eq!(
            merged.content,
            "[tools]\nhk = \"latest\"\nruff = \"latest\"\n\"npm:prettier\" = \"latest\"\n\n[tasks.pre-commit]\nrun = \"hk run pre-commit\"\n"
        );
        assert_eq!(merged.added_tools, ["ruff", "npm:prettier"]);
    }

    #[test]
    fn configured_linter_tools_are_kept() {
        let input = "[tools]\nhk = \"1\"\n\"aqua:astral-sh/ruff\" = \"0.9\"\neslint = \"9\"\n\"github:detekt/detekt\" = { version = \"1\" }\n";
        let merged = merge_mise_config(
            input,
            false,
            &[
                "ruff",
                "npm:eslint",
                "github:detekt/detekt[asset_pattern=detekt-cli-*.zip]",
            ],
        )
        .unwrap();
        assert!(merged.added_tools.is_empty());
        assert!(merged.content.starts_with(input));
    }

    #[test]
    fn language_runtimes_are_not_pinned() {
        let merged = merge_mise_config("", false, &["rust", "taplo"]).unwrap();
        assert!(!merged.content.contains("rust"));
        assert_eq!(merged.added_tools, ["taplo"]);
        assert_eq!(merged.skipped_runtimes, ["rust"]);
    }

    #[test]
    fn configured_runtimes_are_not_reported() {
        let merged = merge_mise_config("[tools]\nrust = \"1.90\"\n", false, &["rust"]).unwrap();
        assert!(merged.skipped_runtimes.is_empty());
    }

    #[test]
    fn builtins_list_their_tools() {
        let ruff = crate::builtins::BUILTINS_META
            .iter()
            .find(|meta| meta.name == "ruff")
            .unwrap();
        assert_eq!(builtin_tools(&[ruff, ruff]), ["ruff"]);
    }
}
