use crate::Result;
use std::process::Command;

#[derive(Debug, clap::Args)]
pub struct NoCommitToBranch {
    /// Branch names to protect (default: main, master)
    #[clap(long, value_delimiter = ',')]
    pub branch: Option<Vec<String>>,
}

impl NoCommitToBranch {
    pub async fn run(&self) -> Result<()> {
        let protected_branches = self
            .branch
            .clone()
            .unwrap_or_else(|| vec!["main".to_string(), "master".to_string()]);

        let current_branch = get_current_branch()?;

        if protected_branches.contains(&current_branch) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "Cannot commit directly to protected branch '{}'",
                    current_branch
                ),
            )
            .into());
        }

        Ok(())
    }
}

fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get current git branch",
        )
        .into());
    }

    let branch = String::from_utf8(output.stdout)?
        .trim()
        .to_string();

    Ok(branch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_current_branch() {
        // This test will only pass in a git repository
        // In CI or non-git environments, it might fail
        let result = get_current_branch();
        if result.is_ok() {
            let branch = result.unwrap();
            // Branch name should not be empty
            assert!(!branch.is_empty());
        }
    }
}
