use crate::Result;
use crate::version;

/// Print the version of hk
#[derive(Debug, usage_derive::Args)]
#[usage(effect = "read")]
pub struct Version {}

impl Version {
    pub async fn run(&self) -> Result<()> {
        println!("{}", version::version());
        Ok(())
    }
}
