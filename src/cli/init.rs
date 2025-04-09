use std::path::PathBuf;

use crate::{Result, env};

/// Generates a new hk.pkl file for a project
#[derive(Debug, clap::Args)]
#[clap(alias = "generate")]
pub struct Init {
    /// Overwrite existing hk.pkl file
    #[clap(short, long)]
    force: bool,
    /// Generate a mise.toml file with hk configured
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
    mise: bool,
}

impl Init {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.pkl");
        let hook_content = r#"
amends "package://github.com/jdx/hk/releases/download/v0.7.0/hk@0.7.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v0.7.0/hk@0.7.0#/builtins.pkl"

local linters = new Mapping<String, Step> {
    // uses builtin prettier linter config
    ["prettier"] = builtins.prettier

    // uses custom pkl linter config
    ["pkl"] = new LinterStep {
        glob = List("*.pkl")
        check = "pkl eval {{files}} >/dev/null"
    }
}

hooks {
    ["pre-commit"] {
        fix = true // automatically modify files with available linter fixes
        steps = new {
            ["stash"] = new StashStep {} // stashes any unstaged changes during the hook
            // "prelint" here is simply a name to define the step
            ["prelint"] {
                // if a step has a "check" script it will execute that
                run = "mise run prelint"
                exclusive = true // ensures that the step runs in isolation
            }
            ...linters // add all linters defined above
            ["postlint"] {
                run = "mise run postlint"
                exclusive = true
            }
        }
    }
    // instead of pre-commit, you can instead define pre-push hooks
    ["pre-push"] {
        steps = linters
    }
    // "fix" and "check" are special steps for `hk fix` and `hk check` commands
    ["fix"] {
        fix = true
        steps = linters
    }
    ["check"] {
        steps = linters
    }
}
"#;
        if !hk_file.exists() || self.force {
            xx::file::write(hk_file, hook_content.trim_start())?;
        } else if !self.force {
            warn!("hk.pkl already exists, run with --force to overwrite");
        }

        if *env::HK_MISE || self.mise {
            let mise_toml = PathBuf::from("mise.toml");
            let mise_content = r#"[tools]
hk = "latest"
pkl = "latest"
"#;
            if mise_toml.exists() {
                warn!("mise.toml already exists, run with --force to overwrite");
            } else {
                xx::file::write(mise_toml, mise_content)?;
                println!("Generated mise.toml");
            }
        }
        Ok(())
    }
}
