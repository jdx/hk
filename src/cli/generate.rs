use std::path::PathBuf;

use crate::{Result, env, version};

/// Generates a new hk.pkl file for a project
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "g", alias = "init")]
pub struct Generate {
    /// Overwrite existing hk.pkl file
    #[clap(short, long)]
    force: bool,
    /// Generate a mise.toml file with hk configured
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
    mise: bool,
}

impl Generate {
    pub async fn run(&self) -> Result<()> {
        let hk_file = PathBuf::from("hk.pkl");
        let version = version::version();
        let hook_content = format!(
            r#"
amends "package://github.com/jdx/hk/releases/download/v0.3.2/hk@0.3.2#/Config.pkl"
// import "package://github.com/jdx/hk/releases/download/v0.3.2/hk@0.3.2#/builtins/prettier.pkl"

min_hk_version = "{version}"

// example git hooks are defined below
//
// `pre-commit` {{
//     // "prelint" here is simply a name to define the step
//     ["prelint"] {{
//         // if a step has a "check" script it will execute that
//         check = "mise run prelint"
//         exclusive = true // ensures that the step runs in isolation
//     }}
//     // everything from here to postlint is run in parallel
//     ["pkl"] {{
//         glob = new {{ "*.pkl" }}
//         check = "pkl eval {{files}} >/dev/null"
//     }}
//     // predefined formatters+linters
//     ["cargo-check"] = new cargo_check.CargoCheck {{}}
//     ["cargo-fmt"] = new cargo_fmt.CargoFmt {{}}
//     ["eslint"] = new eslint.Eslint {{}}
//     ["prettier"] = new prettier.Prettier {{
//         glob = new {{ "*.js"; "*.ts" }} // override the default globs
//     }}
//     ["postlint"] {{
//         check = "mise run postlint"
//         exclusive = true
//     }}
// }}
//
// // instead of pre-commit, you can instead define pre-push hooks
// `pre-push` {{
//     ["eslint"] = new builtins.Eslint {{}}
// }}
//
// // TODO
// `commit-msg` {{
// }}
//
// // TODO
// prepare-commit-msg {{
// }}
//
// // TODO
// update {{
// }}
"#
        );
        xx::file::write(hk_file, hook_content.trim_start())?;

        if *env::HK_MISE || self.mise {
            let mise_toml = PathBuf::from("mise.toml");
            let mise_content = r#"[tools]
hk = "latest"

[tasks]
pre-commit = "hk run pre-commit"
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
