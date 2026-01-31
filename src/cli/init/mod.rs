mod detector;
mod generator;
mod picker;

use std::path::PathBuf;

use crate::{Result, env};

/// Default hooks to configure when none are specified
const DEFAULT_HOOKS: &[&str] = &["pre-commit", "check", "fix"];

/// Generates a new hk.pkl file for a project
#[derive(Debug, clap::Args)]
#[clap(alias = "generate")]
pub struct Init {
    /// Overwrite existing hk.pkl file
    #[clap(short, long)]
    force: bool,
    /// Interactive mode: select linters and hooks manually
    #[clap(short, long)]
    interactive: bool,
    /// Generate a mise.toml file with hk configured
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
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
            println!("Detected: {}", summary);
        }
        println!("Created hk.pkl");

        Ok(())
    }

    fn run_interactive(&self, detections: &[detector::Detection], version: &str) -> Result<String> {
        // Print detection info
        if !detections.is_empty() {
            println!("\nScanning project...");
            for detection in detections {
                println!(
                    "  Detected: {} ({})",
                    detection.builtin.name, detection.reason
                );
            }
            println!();
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
        let mise_toml = PathBuf::from("mise.toml");
        let mise_content = r#"[tools]
hk = "latest"
pkl = "latest"

[tasks.pre-commit]
run = "hk run pre-commit"
"#;
        if mise_toml.exists() && !self.force {
            warn!("mise.toml already exists, run with --force to overwrite");
        } else {
            xx::file::write(mise_toml, mise_content)?;
            println!("Generated mise.toml");
        }
        Ok(())
    }
}
