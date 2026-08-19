use crate::Result;

/// Show the companies sponsoring hk and the jdx.dev open source tools
#[derive(Debug, usage_derive::Args)]
#[usage(effect = "read")]
pub struct Sponsors {}

impl Sponsors {
    pub async fn run(&self) -> Result<()> {
        println!(
            "hk and the jdx.dev open source tools are sponsored by:\n\n  entire.io - https://entire.io\n  37signals - https://37signals.com\n\nView all sponsors: https://jdx.dev/sponsors.html"
        );
        Ok(())
    }
}
