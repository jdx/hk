use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::string::ToString;

use crate::Result;

#[derive(Debug, clap::Args)]
pub struct CheckConventionalCommit {
    /// Commit message file to check
    #[clap(required = true)]
    pub commit_msg_file: PathBuf,

    #[clap(long, default_value = default_allowed_types(), value_delimiter = ',')]
    pub allowed_types: Vec<String>,
}

impl CheckConventionalCommit {
    pub async fn run(&self) -> Result<()> {
        check_conventional_commit(&self.commit_msg_file, &self.allowed_types)
    }
}

fn check_conventional_commit(path: &PathBuf, allowed_types: &[String]) -> Result<()> {
    let file = File::open(path)?;
    let mut lines = BufReader::new(file)
        .lines()
        .map_while(StdResult::ok)
        .filter(|line| !line.starts_with('#'));

    let Some(title) = lines.next() else {
        return Err(eyre::eyre!("Empty commit message"));
    };

    parse_commit_title(&title, allowed_types)?;

    Ok(())
}

fn parse_commit_title(title: &str, allowed_types: &[String]) -> Result<bool> {
    // Per conventional commit spec:
    //
    // 1. Commits MUST be prefixed with a type, which consists of a noun, feat, fix, etc.,
    // followed by the OPTIONAL scope, OPTIONAL !, and REQUIRED terminal colon and space.
    // ...
    // 5. A description MUST immediately follow the colon and space after the type/scope prefix.
    // The description is a short summary of the code changes, e.g.,
    // fix: array parsing issue when multiple spaces were contained in string.
    let mut parts = title.splitn(2, ":");

    // Ensure commit type is provided and isn't an empty string
    let prefix = if let Some(prefix) = parts.next()
        && !prefix.is_empty()
    {
        prefix
    } else {
        return Err(eyre::eyre!("Missing commit type"));
    };
    let mut type_and_scope = prefix.trim_end_matches('!').splitn(2, '(');
    let Some(commit_type) = type_and_scope.next() else {
        return Err(eyre::eyre!("Missing commit type"));
    };

    if !check_commit_type(commit_type, allowed_types) {
        return Err(eyre::eyre!("Invalid commit type: '{commit_type}'"));
    }

    if let Some(scope) = type_and_scope.next()
        && !scope.ends_with(')')
    {
        return Err(eyre::eyre!("Invalid scope, missing closing parentheses"));
    }

    // Ensure description has been provided and isn't an empty string
    let Some(description) = parts.next() else {
        return Err(eyre::eyre!("Missing description"));
    };
    if description.strip_prefix(' ').unwrap_or_default().is_empty() {
        return Err(eyre::eyre!("Missing description"));
    }

    return Ok(true);
}

fn check_commit_type(commit_type: &str, allowed_types: &[String]) -> bool {
    return allowed_types.contains(&commit_type.to_string());
}

fn default_allowed_types() -> String {
    return [
        "build", "chore", "ci", "docs", "feat", "fix", "perf", "refactor", "revert", "style",
        "test",
    ]
    .join(",");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_empty_commit_message() {
        let commit_msg_file = NamedTempFile::new().unwrap();
        let path = commit_msg_file.path().to_path_buf();
        let result = check_conventional_commit(&path, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Empty commit message");
    }

    #[test]
    fn test_missing_commit_type() {
        let commit_msg_file = NamedTempFile::new().unwrap();
        let path = commit_msg_file.path().to_path_buf();
        fs::write(&path, b": test description").unwrap();

        let result = check_conventional_commit(&path, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Missing commit type");
    }

    #[test]
    fn test_missing_description() {
        let commit_msg_file = NamedTempFile::new().unwrap();
        let path = commit_msg_file.path().to_path_buf();
        fs::write(&path, b"test: ").unwrap();

        let result = check_conventional_commit(&path, &["test".to_string()]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Missing description");
    }

    #[test]
    fn test_invalid_commit_type() {
        let commit_msg_file = NamedTempFile::new().unwrap();
        let path = commit_msg_file.path().to_path_buf();
        fs::write(&path, b"testing: test description").unwrap();

        let result = check_conventional_commit(&path, &["test".to_string()]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid commit type: 'testing'"
        );
    }

    #[test]
    fn test_unenclosed_scope_parentheses() {
        let commit_msg_file = NamedTempFile::new().unwrap();
        let path = commit_msg_file.path().to_path_buf();
        fs::write(&path, b"test(scope: test description").unwrap();

        let result = check_conventional_commit(&path, &["test".to_string()]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid scope, missing closing parentheses"
        );
    }

    #[test]
    fn test_valid_commit_message() {
        let commit_msg_file = NamedTempFile::new().unwrap();
        let path = commit_msg_file.path().to_path_buf();
        fs::write(&path, b"test(scope): test description").unwrap();

        let result = check_conventional_commit(&path, &["test".to_string()]);
        assert!(result.is_ok());
    }
}
