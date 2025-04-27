use crate::Result;

/// Lists all available builtin linters
#[derive(Debug, clap::Args)]
pub struct Builtins;
include!(concat!(env!("OUT_DIR"), "/builtins.rs"));

impl Builtins {
    pub async fn run(&self) -> Result<()> {
        for builtin in BUILTINS {
            println!("{}", builtin);
        }

        Ok(())
    }
}
