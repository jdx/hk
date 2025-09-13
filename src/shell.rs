use std::process::Command;
use eyre::Result;

#[derive(Debug, Clone)]
pub enum Shell {
    Sh,           // Unix shell (sh -o errexit -c)
    PowerShell,   // Windows PowerShell or PowerShell Core
    Cmd,          // Windows Command Prompt
}

impl Shell {
    pub fn detect() -> Self {
        if cfg!(windows) {
            if which::which("powershell.exe").is_ok() || which::which("pwsh.exe").is_ok() {
                Shell::PowerShell
            } else {
                Shell::Cmd
            }
        } else {
            // On Unix, always use sh for consistency
            Shell::Sh
        }
    }

    pub fn command(&self) -> Command {
        match self {
            Shell::Sh => {
                let mut cmd = Command::new("sh");
                cmd.arg("-o").arg("errexit").arg("-c");
                cmd
            }
            Shell::PowerShell => {
                let mut cmd = if which::which("pwsh.exe").is_ok() {
                    Command::new("pwsh.exe")
                } else {
                    Command::new("powershell.exe")
                };
                cmd.arg("-NoProfile")
                    .arg("-NonInteractive")
                    .arg("-Command");
                cmd
            }
            Shell::Cmd => {
                let mut cmd = Command::new("cmd.exe");
                cmd.arg("/C");
                cmd
            }
        }
    }
    
    /// Create a CmdLineRunner configured for this shell type
    pub fn runner(&self) -> ensembler::CmdLineRunner {
        use ensembler::CmdLineRunner;
        
        match self {
            Shell::Sh => CmdLineRunner::new("sh").arg("-o").arg("errexit").arg("-c"),
            Shell::PowerShell => {
                if which::which("pwsh.exe").is_ok() {
                    CmdLineRunner::new("pwsh.exe")
                } else {
                    CmdLineRunner::new("powershell.exe")
                }
                .arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-Command")
            }
            Shell::Cmd => CmdLineRunner::new("cmd.exe").arg("/C"),
        }
    }

    pub fn execute(&self, script: &str) -> Result<String> {
        let mut cmd = self.command();
        cmd.arg(script);
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(eyre::eyre!("Command failed: {:?}\nstderr: {}", cmd, stderr).into());
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

}

