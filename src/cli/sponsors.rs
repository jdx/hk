use crate::Result;

/// Show the companies sponsoring hk and the en.dev project family
#[derive(Debug, clap::Args)]
pub struct Sponsors {}

impl Sponsors {
    pub async fn run(&self) -> Result<()> {
        println!(
            "hk and the en.dev project family are sponsored by:\n\n  37signals - https://37signals.com\n\nView all sponsors: https://en.dev/sponsors.html"
        );
        Ok(())
    }
}
