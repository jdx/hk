use crate::Result;
use std::path::PathBuf;
use std::process::Command;

const PERMS_GITLINK: &str = "160000";

#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct ForbidSubmodules {
    /// Paths to check. Defaults to the whole repository.
    #[usage(arg)]
    pub paths: Option<Vec<PathBuf>>,
}

impl ForbidSubmodules {
    pub async fn run(&self) -> Result<()> {
        let submodules = find_submodules(self.paths.as_deref().unwrap_or_default())?;
        if submodules.is_empty() {
            return Ok(());
        }

        for path in &submodules {
            println!("{path}: submodules are not allowed in this repository");
        }
        println!();
        println!("To fix this, run `git rm <submodule>`.");
        println!("Also check `.gitmodules` for any unintended entries.");

        Err(eyre::eyre!("Submodules found"))
    }
}

fn find_submodules(paths: &[PathBuf]) -> Result<Vec<String>> {
    let mut args = vec![
        "ls-files".to_string(),
        "--stage".to_string(),
        "-z".to_string(),
        "--".to_string(),
    ];
    if paths.is_empty() {
        args.push(".".to_string());
    } else {
        args.extend(paths.iter().map(|p| p.to_string_lossy().into_owned()));
    }

    let output = Command::new("git").args(&args).output()?;
    if !output.status.success() {
        return Err(eyre::eyre!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(collect_gitlinks(&String::from_utf8_lossy(&output.stdout)))
}

/// Reads `<mode> <object> <stage>\t<path>` records from `git ls-files --stage -z`.
fn collect_gitlinks(stdout: &str) -> Vec<String> {
    stdout
        .split('\0')
        .filter_map(|record| {
            let (metadata, path) = record.split_once('\t')?;
            metadata
                .starts_with(PERMS_GITLINK)
                .then(|| path.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_gitlinks_finds_submodules() {
        let stdout = "100644 c7c3583c 0\t.gitmodules\0\
                      160000 f46e527b 0\tvendor/dep\0\
                      100644 587be6b4 0\tpath with spaces.txt\0\
                      160000 a1b2c3d4 0\tproject2/sub module\0";

        assert_eq!(
            collect_gitlinks(stdout),
            vec!["vendor/dep", "project2/sub module"]
        );
    }

    #[test]
    fn test_collect_gitlinks_ignores_regular_files() {
        let stdout = "100644 c7c3583c 0\t.gitmodules\0\
                      100755 abcdef01 0\tscript.sh\0";

        assert!(collect_gitlinks(stdout).is_empty());
    }

    #[test]
    fn test_collect_gitlinks_ignores_empty_output() {
        assert!(collect_gitlinks("").is_empty());
    }
}
