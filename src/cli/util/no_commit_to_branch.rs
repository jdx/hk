use crate::Result;
use std::path::Path;
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

        if let Some(current_branch) = get_current_branch()?
            && protected_branches.contains(&current_branch)
        {
            return Err(std::io::Error::other(format!(
                "Cannot commit directly to protected branch '{}'",
                current_branch
            ))
            .into());
        }

        Ok(())
    }
}

fn get_current_branch() -> Result<Option<String>> {
    get_current_branch_in(Path::new("."))
}

fn get_current_branch_in(dir: &Path) -> Result<Option<String>> {
    // Use symbolic-ref instead of rev-parse to work in repos without commits
    let output = Command::new("git")
        .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
        .current_dir(dir)
        .output()?;

    if output.status.success() {
        let branch = String::from_utf8(output.stdout)?.trim().to_string();
        return Ok(Some(branch));
    }

    // A detached HEAD is expected during operations such as interactive rebases.
    if output.status.code() == Some(1) {
        return Ok(None);
    }

    Err(std::io::Error::other("Failed to get current git branch").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(dir)
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    fn test_get_current_branch_attached_and_detached() {
        let dir = tempfile::tempdir().unwrap();
        git(dir.path(), &["init", "-q"]);
        git(dir.path(), &["checkout", "-q", "-b", "feature"]);

        assert_eq!(
            get_current_branch_in(dir.path()).unwrap(),
            Some("feature".to_string())
        );

        git(
            dir.path(),
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "--allow-empty",
                "-qm",
                "initial",
            ],
        );
        git(dir.path(), &["checkout", "-q", "--detach", "HEAD"]);

        assert_eq!(get_current_branch_in(dir.path()).unwrap(), None);
    }

    #[test]
    fn test_get_current_branch_errors_outside_repository() {
        let dir = tempfile::tempdir().unwrap();
        assert!(get_current_branch_in(dir.path()).is_err());
    }
}
