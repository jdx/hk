mod detector;
mod generator;
mod picker;

use std::path::PathBuf;

use crate::{Result, env};

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
    /// Choose the curated tool setup (fast or ecosystem)
    #[usage(long, choices("fast", "ecosystem"))]
    preset: Option<String>,
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
            self.run_auto(&detections, &project_root, version)
        };

        // Write the file
        xx::file::write(&hk_file, &hook_content)?;

        // Print summary
        if !detections.is_empty() && !self.interactive && self.preset.is_none() {
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

        let preference = match &self.preset {
            Some(preset) => preset.clone(),
            None => picker::pick_preset()?,
        };
        let builtins = if preference == "custom" {
            picker::pick_builtins(detections)?
        } else {
            let selected = detector::preset_builtins_with_detections(
                &std::env::current_dir()?,
                detections,
                &preference,
            );
            self.report_selection(&selected, &preference);
            if !picker::confirm_preset()? {
                picker::pick_builtins_with_selected(&selected)?
            } else {
                selected
            }
        };

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

    fn report_selection(&self, selected: &[&crate::builtins::BuiltinMeta], preset: &str) {
        info!(
            "Recommended ({preset}): {}",
            selected
                .iter()
                .map(|m| m.name)
                .collect::<Vec<_>>()
                .join(", ")
        );
        info!(
            "These steps configure hk only; install each tool and configure its native rules separately."
        );
        if selected.iter().any(|m| m.name == "biome")
            && selected.iter().any(|m| m.name == "prettier")
        {
            warn!(
                "Biome and Prettier both cover formatting; review the generated steps to avoid overlap."
            );
        }
    }

    fn run_auto(
        &self,
        detections: &[detector::Detection],
        project_root: &std::path::Path,
        version: &str,
    ) -> String {
        let builtins: Vec<_> = if let Some(preset) = &self.preset {
            let selected =
                detector::preset_builtins_with_detections(project_root, detections, preset);
            self.report_selection(&selected, preset);
            if selected.is_empty() {
                return generator::generate_default_template(version);
            }
            selected
        } else {
            if detections.is_empty() {
                return generator::generate_default_template(version);
            }
            detections.iter().map(|d| d.builtin).collect()
        };
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
            info!("Generated mise.toml");
        }
        Ok(())
    }
}
