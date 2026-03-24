use crate::Result;
use std::path::PathBuf;

#[derive(Debug, clap::Args)]
pub struct TyposDiff {
    /// Files to check
    #[clap(required = true)]
    pub files: Vec<PathBuf>,
}

impl TyposDiff {
    pub async fn run(&self) -> Result<()> {
        let mut cmd = xx::process::cmd("typos", ["--force-exclude", "--diff"])
            .stdout_capture()
            .stderr_capture()
            .unchecked();
        for file in &self.files {
            cmd = cmd.arg(file.as_os_str());
        }

        let output = cmd.run()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            if stderr.trim().is_empty() {
                eyre::bail!("typos exited with status {}", output.status);
            }
            eyre::bail!(stderr.trim().to_string());
        }

        if stdout.is_empty() {
            return Ok(());
        }

        print!("{stdout}");
        std::process::exit(1);
    }
}
