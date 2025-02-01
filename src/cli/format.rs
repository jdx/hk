use std::path::PathBuf;
use crate::core;

use xx::file;

use crate::Result;

#[derive(Debug, clap::Parser)]
pub struct Format {
    /// The formatter(s) to use
    #[clap(required = true)]
    pub formatter: Vec<String>,
    /// The input files to format
    #[clap(long, short, conflicts_with = "all", required_unless_present = "all")]
    pub file: Option<Vec<PathBuf>>,
    /// Format all files in the repository
    #[clap(short, long)]
    pub all: bool,
}

impl Format {
    pub async fn run(&self) -> Result<()> {
        static EMPTY: Vec<PathBuf> = vec![];
        let files = self.file.as_ref().unwrap_or(&EMPTY);
        for file in files {
            let mut code = file::read_to_string(file)?;
            for id in &self.formatter {
                let plugin = core::get(id);
                code = plugin.format(&code, file).await?;
            }
            file::write(file, code.as_bytes())?;
        }
        Ok(())
    }
}
