use crate::Result;
use crate::builtins::BUILTINS;

/// Lists all available builtin linters
#[derive(Debug, clap::Args)]
pub struct Builtins;

impl Builtins {
    pub async fn run(&self) -> Result<()> {
        for builtin in BUILTINS {
            println!("{builtin}");
        }

        Ok(())
    }
}
