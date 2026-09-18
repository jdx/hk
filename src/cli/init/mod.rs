mod detector;
mod generator;
mod picker;

use std::path::PathBuf;

use crate::{Result, env};
use eyre::eyre;
use toml_edit::{DocumentMut, Item, Table, value};

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
        let has_file_task = existing_pre_commit_file_task(&std::env::current_dir()?);
        let content = merge_mise_config(original.as_deref().unwrap_or(""), has_file_task)?;
        if original.as_deref() != Some(content.as_str()) {
            xx::file::write(&mise_file, content)?;
            info!("Updated mise.toml");
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
    let has_hk = tools.iter().any(|(key, _)| {
        matches!(
            key,
            "hk" | "aqua:jdx/hk" | "aqua:hk" | "ubi:jdx/hk" | "cargo:hk" | "github:jdx/hk"
        )
    });
    if !has_hk {
        tools.insert("hk", value("latest"));
    }
    let has_pkl = tools.iter().any(|(key, _)| {
        matches!(
            key,
            "pkl" | "aqua:pkl" | "aqua:apple/pkl" | "ubi:apple/pkl" | "github:apple/pkl"
        )
    });
    if !has_pkl {
        tools.insert("pkl", value("latest"));
    }
    if document.get("tasks").is_none() {
        document["tasks"] = Item::Table(Table::new());
    }
    let includes = document
        .get("task_config")
        .and_then(Item::as_table_like)
        .and_then(|table| table.get("includes"))
        .is_some();
    let tasks = document["tasks"]
        .as_table_like_mut()
        .ok_or_else(|| eyre!("unsupported mise.toml: [tasks] must be a table"))?;
    if tasks.get("pre-commit").is_none() && !external_pre_commit && !includes {
        tasks.insert(
            "pre-commit",
            Item::Table(Table::from_iter([("run", value("hk run pre-commit"))])),
        );
    } else if tasks.get("pre-commit").is_none() && (external_pre_commit || includes) {
        warn!(
            "Preserving external mise task configuration for pre-commit; wire hk manually if needed"
        );
    }
    Ok(document.to_string())
}
