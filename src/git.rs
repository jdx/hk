//! Git repository operations
//!
//! This module provides high-level git operations for working with repositories,
//! including cloning, fetching, and querying repository state.
//!
//! ## Features
//!
//! - Clone repositories with custom options
//! - Query current branch, SHA, and remote URL
//! - Update repositories with fetch and checkout
//! - Automatic safe.directory configuration
//!
//! ## Examples
//!
//! ### Cloning a repository
//!
//! ```rust,no_run
//! use xx::git::{clone, CloneOptions};
//!
//! # fn main() -> xx::XXResult<()> {
//! // Clone with default options (shallow clone of default branch)
//! let repo = clone("https://github.com/rust-lang/rust", "/tmp/rust", &CloneOptions::default())?;
//!
//! // Clone a specific branch
//! let options = CloneOptions::default().branch("stable");
//! let repo = clone("https://github.com/rust-lang/rust", "/tmp/rust-stable", &options)?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Working with existing repositories
//!
//! ```rust,no_run
//! use xx::git::Git;
//! use std::path::PathBuf;
//!
//! # fn main() -> xx::XXResult<()> {
//! let git = Git::new(PathBuf::from("/path/to/repo"));
//!
//! // Check if directory is a git repository
//! if git.is_repo() {
//!     // Get current branch name
//!     let branch = git.current_branch()?;
//!     println!("Current branch: {}", branch);
//!
//!     // Get current commit SHA
//!     let sha = git.current_sha()?;
//!     println!("Current SHA: {}", sha);
//!
//!     // Get remote URL
//!     if let Some(url) = git.get_remote_url() {
//!         println!("Remote URL: {}", url);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use std::{
    path::{Path, PathBuf},
    vec,
};

use duct::{Expression, cmd};
use miette::{Result, miette};

use crate::{XXError, XXResult, file};

/// A git repository handle
pub struct Git {
    /// The directory containing the git repository
    pub dir: PathBuf,
}

macro_rules! git_cmd {
    ( $dir:expr $(, $arg:expr )* $(,)? ) => {
        {
            let safe = format!("safe.directory={}", $dir.display());
            cmd!("git", "-C", $dir, "-c", safe $(, $arg)*)
        }
    }
}

impl Git {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn is_repo(&self) -> bool {
        self.dir.join(".git").is_dir()
    }

    pub fn update(&self, gitref: Option<String>) -> Result<(String, String)> {
        let gitref = gitref.map_or_else(|| self.current_branch(), Ok)?;
        debug!("updating {} to {}", self.dir.display(), gitref);
        let exec = |cmd: Expression| match cmd.stderr_to_stdout().stdout_capture().unchecked().run()
        {
            Ok(res) => {
                if res.status.success() {
                    Ok(())
                } else {
                    Err(miette!(
                        "git failed: {cmd:?} {}",
                        String::from_utf8(res.stdout).unwrap()
                    ))
                }
            }
            Err(err) => Err(miette!("git failed: {cmd:?} {err:#}")),
        };
        exec(git_cmd!(
            &self.dir,
            "fetch",
            "--prune",
            "--update-head-ok",
            "origin",
            &format!("{gitref}:{gitref}"),
        ))?;
        let prev_rev = self.current_sha()?;
        exec(git_cmd!(
            &self.dir,
            "-c",
            "advice.detachedHead=false",
            "-c",
            "advice.objectNameWarning=false",
            "checkout",
            "--force",
            &gitref
        ))?;
        let post_rev = self.current_sha()?;
        file::touch_dir(&self.dir)?;

        Ok((prev_rev, post_rev))
    }

    pub fn current_branch(&self) -> XXResult<String> {
        let branch = git_cmd!(&self.dir, "branch", "--show-current")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current branch for {}: {}", self.dir.display(), &branch);
        Ok(branch)
    }
    pub fn current_sha(&self) -> XXResult<String> {
        let sha = git_cmd!(&self.dir, "rev-parse", "HEAD")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current sha for {}: {}", self.dir.display(), &sha);
        Ok(sha)
    }

    pub fn current_sha_short(&self) -> XXResult<String> {
        let sha = git_cmd!(&self.dir, "rev-parse", "--short", "HEAD")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current sha for {}: {}", self.dir.display(), &sha);
        Ok(sha)
    }

    pub fn current_abbrev_ref(&self) -> XXResult<String> {
        let aref = git_cmd!(&self.dir, "rev-parse", "--abbrev-ref", "HEAD")
            .read()
            .map_err(|err| XXError::GitError(err, self.dir.clone()))?;
        debug!("current abbrev ref for {}: {}", self.dir.display(), &aref);
        Ok(aref)
    }

    pub fn get_remote_url(&self) -> Option<String> {
        if !self.dir.exists() {
            return None;
        }
        let res = git_cmd!(&self.dir, "config", "--get", "remote.origin.url").read();
        match res {
            Ok(url) => {
                debug!("remote url for {}: {}", self.dir.display(), &url);
                Some(url)
            }
            Err(err) => {
                warn!(
                    "failed to get remote url for {}: {:#}",
                    self.dir.display(),
                    err
                );
                None
            }
        }
    }

    pub fn split_url_and_ref(url: &str) -> (String, Option<String>) {
        match url.split_once('#') {
            Some((url, _ref)) => (url.to_string(), Some(_ref.to_string())),
            None => (url.to_string(), None),
        }
    }
}

pub fn clone<D: AsRef<Path>>(url: &str, dir: D, clone_options: &CloneOptions) -> XXResult<Git> {
    let dir = dir.as_ref().to_path_buf();
    debug!("cloning {} to {}", url, dir.display());
    if let Some(parent) = dir.parent() {
        file::mkdirp(parent)?;
    }
    match get_git_version() {
        Ok(version) => trace!("git version: {version}"),
        Err(err) => warn!("failed to get git version: {err:#}\n Git is required to use mise."),
    }

    let dir_str = dir.to_string_lossy().to_string();
    let mut cmd_args = vec!["clone", "-q", "--depth", "1", &url, &dir_str];

    if let Some(branch) = clone_options.branch.as_ref() {
        cmd_args.push("--branch");
        cmd_args.push(branch);
        cmd_args.push("--single-branch");
        cmd_args.push("-c");
        cmd_args.push("advice.detachedHead=false");
    }

    cmd("git", &cmd_args)
        .run()
        .map_err(|err| XXError::GitError(err, dir.clone()))?;

    Ok(Git::new(dir))
}

fn get_git_version() -> Result<String> {
    let version = cmd!("git", "--version")
        .read()
        .map_err(|err| XXError::GitError(err, PathBuf::new()))?;
    Ok(version.trim().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_git() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        assert!(!git.is_repo());
        assert_eq!(git.get_remote_url(), None);
        assert!(git.current_branch().is_err());
        assert!(git.current_sha().is_err());
        assert!(git.current_sha_short().is_err());
        assert!(git.current_abbrev_ref().is_err());

        let git = clone(
            "https://github.com/jdx/xx",
            &git.dir,
            &CloneOptions::default(),
        )
        .unwrap();
        assert!(git.is_repo());
        assert_eq!(
            git.get_remote_url(),
            Some("https://github.com/jdx/xx".to_string())
        );

        file::remove_dir_all(tmp.path()).unwrap();
    }

    #[test]
    fn test_git_with_options() {
        let tmp = tempfile::tempdir().unwrap();
        let git = Git::new(tmp.path().to_path_buf());
        assert!(!git.is_repo());
        assert_eq!(git.get_remote_url(), None);
        assert!(git.current_branch().is_err());
        assert!(git.current_sha().is_err());
        assert!(git.current_sha_short().is_err());
        assert!(git.current_abbrev_ref().is_err());

        let clone_options = CloneOptions::default().branch("v2.0.0");

        let git = clone("https://github.com/jdx/xx", &git.dir, &clone_options).unwrap();
        assert!(git.is_repo());
        assert!(
            git.current_sha()
                .is_ok_and(|s| s == "e5352617769f0edff7758713d05fff6b6ddf1266")
        );
        assert_eq!(
            git.get_remote_url(),
            Some("https://github.com/jdx/xx".to_string())
        );

        file::remove_dir_all("/tmp/xx").unwrap();
    }
}

#[derive(Default)]
pub struct CloneOptions {
    branch: Option<String>,
}

impl CloneOptions {
    pub fn branch(mut self, branch: &str) -> Self {
        self.branch = Some(branch.to_string());
        self
    }
}
