mod detector;
mod generator;
mod picker;

use std::path::PathBuf;

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
    /// Set HK_MISE=1 to make this the default behavior.
    #[usage(long, verbatim_doc_comment)]
    mise: bool,
}

impl Init {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.pkl");
        let version = env!("CARGO_PKG_VERSION");

        // Handle mise.toml generation first (independent of hk.pkl)
        if *env::HK_MISE || self.mise {
            self.write_mise_toml()?;
        }

        // Check if file exists and handle --force flag
        if hk_file.exists() && !self.force {
            warn!("hk.pkl already exists, run with --force to overwrite");
            return Ok(());
        }

        // Detect project files
        let project_root = std::env::current_dir()?;
        let detections = detector::detect_builtins(&project_root);

        let hook_content = if self.interactive {
            // Interactive mode: let user pick from all builtins
            self.run_interactive(&detections, version)?
        } else {
            // Auto mode (default): use detected builtins or fall back to template
            self.run_auto(&detections, version)
        };

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

        Ok(())
    }

    fn run_interactive(&self, detections: &[detector::Detection], version: &str) -> Result<String> {
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
            return Ok(generator::generate_default_template(version));
        }

        // Let user pick hooks
        let hooks = picker::pick_hooks()?;

        let hooks = if hooks.is_empty() {
            warn!("No hooks selected, using defaults");
            DEFAULT_HOOKS.iter().map(|s| s.to_string()).collect()
        } else {
            hooks
        };

        Ok(generator::generate_pkl(&builtins, &hooks, version))
    }

    fn run_auto(&self, detections: &[detector::Detection], version: &str) -> String {
        if detections.is_empty() {
            // No detections - use default template
            return generator::generate_default_template(version);
        }

        // Use detected builtins with default hooks
        let builtins: Vec<_> = detections.iter().map(|d| d.builtin).collect();
        let hooks: Vec<String> = DEFAULT_HOOKS.iter().map(|s| s.to_string()).collect();

        generator::generate_pkl(&builtins, &hooks, version)
    }

    fn write_mise_toml(&self) -> Result<()> {
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
        let content = merge_mise_config(input, suppress_pre_commit)?;
        if original.as_deref() != Some(content.as_str()) {
            xx::file::write(&mise_file, content)?;
            if original.is_none() {
                info!("Generated mise.toml");
            } else {
                info!("Updated mise.toml");
            }
        } else {
            info!("mise.toml already contains hk configuration");
        }
        Ok(())
    }
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
            for candidate in [
                task.clone(),
                task.join("_default"),
                path.join("pre-commit.toml"),
            ] {
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

/// Merge hk's minimal mise entries without replacing user configuration.
fn merge_mise_config(input: &str, external_pre_commit: bool) -> Result<String> {
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
    let has_pre_commit = match document.get("tasks") {
        Some(item) => item
            .as_table_like()
            .ok_or_else(|| eyre!("unsupported mise.toml: [tasks] must be a table"))?
            .get("pre-commit")
            .is_some(),
        None => false,
    };
    if !has_pre_commit {
        if external_pre_commit {
            warn!(
                "Preserving external mise task configuration for pre-commit; wire hk manually if needed"
            );
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
    Ok(document.to_string())
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
            merge_mise_config("", false).unwrap(),
            "[tools]\nhk = \"latest\"\n\n[tasks.pre-commit]\nrun = \"hk run pre-commit\"\n"
        );
    }

    #[test]
    fn dotted_tools_and_implicit_tasks_have_stable_output() {
        assert_eq!(
            merge_mise_config("tools.node = \"20\"\n", false).unwrap(),
            "tools.node = \"20\"\ntools.hk = \"latest\"\n\n[tasks.pre-commit]\nrun = \"hk run pre-commit\"\n"
        );
    }

    #[test]
    fn inline_tables_have_stable_output() {
        assert_eq!(
            merge_mise_config(
                "tools = { node = \"20\" }\ntasks = { check = \"custom\" }\n",
                false,
            )
            .unwrap(),
            "tools = { node = \"20\" , hk = \"latest\" }\ntasks = { check = \"custom\" , pre-commit = { run = \"hk run pre-commit\" } }\n"
        );
    }

    #[test]
    fn scalar_tasks_are_rejected() {
        assert!(merge_mise_config("tasks = \"custom\"\n", false).is_err());
    }

    #[test]
    fn backend_qualified_hk_is_recognized() {
        let output = merge_mise_config(
            "[tools]\n\"asdf:hk\" = \"1\"\n\"vfox:jdx/hk\" = \"2\"\n\"asdf:other\" = \"3\"\n",
            false,
        )
        .unwrap();
        assert!(output.contains("\"asdf:hk\" = \"1\""));
        assert!(output.contains("\"vfox:jdx/hk\" = \"2\""));
        assert!(!output.contains("\nhk = \"latest\""));
    }
}
