use std::process::Command;
use eyre::Result;
use shell_quote::QuoteInto;

#[derive(Debug, Clone)]
pub enum Shell {
    Sh,
    Bash,
    Zsh,
    Fish,
    Dash,
    PowerShell,
    Cmd,
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
            if let Ok(shell) = std::env::var("SHELL") {
                if shell.contains("bash") {
                    Shell::Bash
                } else if shell.contains("zsh") {
                    Shell::Zsh
                } else if shell.contains("fish") {
                    Shell::Fish
                } else if shell.contains("dash") {
                    Shell::Dash
                } else {
                    Shell::Sh
                }
            } else {
                Shell::Sh
            }
        }
    }

    pub fn command(&self) -> Command {
        match self {
            Shell::Sh => {
                let mut cmd = Command::new("sh");
                cmd.arg("-o").arg("errexit").arg("-c");
                cmd
            }
            Shell::Bash => {
                let mut cmd = Command::new("bash");
                cmd.arg("-e").arg("-c");
                cmd
            }
            Shell::Zsh => {
                let mut cmd = Command::new("zsh");
                cmd.arg("-e").arg("-c");
                cmd
            }
            Shell::Fish => {
                let mut cmd = Command::new("fish");
                cmd.arg("-c");
                cmd
            }
            Shell::Dash => {
                let mut cmd = Command::new("dash");
                cmd.arg("-e").arg("-c");
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
            _ => CmdLineRunner::new("sh").arg("-o").arg("errexit").arg("-c"),
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

    pub fn quote(&self, s: &str) -> String {
        match self {
            Shell::PowerShell => {
                if s.contains(' ') || s.contains('"') || s.contains('\'') {
                    format!("'{}'", s.replace('\'', "''"))
                } else {
                    s.to_string()
                }
            }
            Shell::Cmd => {
                if s.contains(' ') || s.contains('"') {
                    format!("\"{}\"", s.replace('"', "\"\""))
                } else {
                    s.to_string()
                }
            }
            Shell::Bash | Shell::Zsh => {
                let mut quoted = String::new();
                shell_quote::Bash::quote_into(s, &mut quoted);
                quoted
            }
            Shell::Fish => {
                let mut quoted = String::new();
                shell_quote::Fish::quote_into(s, &mut quoted);
                quoted
            }
            Shell::Sh | Shell::Dash => {
                let mut quoted = Vec::new();
                shell_quote::Sh::quote_into(s, &mut quoted);
                String::from_utf8(quoted).unwrap_or_default()
            }
        }
    }

    pub fn shebang(&self) -> &str {
        match self {
            Shell::Sh => "#!/bin/sh",
            Shell::Bash => "#!/usr/bin/env bash",
            Shell::Zsh => "#!/usr/bin/env zsh",
            Shell::Fish => "#!/usr/bin/env fish",
            Shell::Dash => "#!/usr/bin/env dash",
            Shell::PowerShell => "#!powershell",
            Shell::Cmd => "@echo off",
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            Shell::PowerShell => "ps1",
            Shell::Cmd => "bat",
            _ => "sh",
        }
    }
}

