use crate::Result;
use crate::builtins::BUILTINS;

/// List all available builtin linters
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Builtins;

impl Builtins {
    pub async fn run(&self) -> Result<()> {
        for builtin in BUILTINS {
            println!("{builtin}");
        }

        Ok(())
    }
}
