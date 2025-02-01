use std::path::PathBuf;

use crate::Result;
use git2::{Repository, StashFlags, StatusOptions, StatusShow};
use itertools::Itertools;

pub struct Git {
    repo: Repository,
    stash_id: Option<git2::Oid>,
    root: PathBuf,
}

impl Git {
    pub fn new() -> Result<Self> {
        let repo = Repository::open(".")?;
        Ok(Self {
            root: repo.workdir().unwrap().to_path_buf(),
            repo,
            stash_id: None,
        })
    }

    pub fn staged_files(&self) -> Result<Vec<PathBuf>> {
        let mut status_options = StatusOptions::new();
        status_options.show(StatusShow::Index);
        let statuses = self.repo.statuses(Some(&mut status_options))?;
        let paths = statuses
            .iter()
            .filter_map(|s| s.path().map(PathBuf::from))
            .filter(|p| p.exists())
            .collect_vec();
        Ok(paths)
    }

    pub fn unstaged_files(&self) -> Result<Vec<PathBuf>> {
        let mut status_options = StatusOptions::new();
        status_options
            .include_untracked(true)
            .show(StatusShow::Workdir);
        let statuses = self.repo.statuses(Some(&mut status_options))?;
        let paths = statuses
            .iter()
            .filter_map(|s| s.path().map(PathBuf::from))
            .collect_vec();
        Ok(paths)
    }

    pub fn stash_unstaged(&mut self) -> Result<()> {
        // Skip stashing if there's no initial commit yet
        if self.repo.head().is_err() {
            return Ok(());
        }
        
        if !self.unstaged_files()?.is_empty() {
            self.push_stash()?;
        }
        Ok(())
    }

    fn push_stash(&mut self) -> Result<()> {
        let stasher = self.repo.signature()?;
        let stash_flags = StashFlags::KEEP_INDEX | StashFlags::INCLUDE_UNTRACKED;
        let stash_id =
            self.repo
                .stash_save(&stasher, "hk pre-commit stash", Some(stash_flags))?;
        self.stash_id = Some(stash_id);
        Ok(())
    }

    pub fn add(&mut self, pathspecs: &[&str]) -> Result<()> {
        let pathspecs = pathspecs
            .iter()
            .map(|p| p.replace(self.root.to_str().unwrap(), ""))
            .collect_vec();
        info!("adding files: {:?}", &pathspecs);
        let mut index = self.repo.index()?;
        index.add_all(&pathspecs, git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    pub fn pop_stash(&mut self) -> Result<()> {
        if self.stash_id.is_none() {
            return Ok(());
        };

        // TODO: figure out how to pop the stash with untracked files using git2
        duct::cmd!("git", "stash", "pop").run()?;
        // let stash_id = self.stash_id.unwrap();

        // // Find the stash index by its ID
        // let mut stash_index = None;
        // self.repo.stash_foreach(|index, _, id| {
        //     if *id == stash_id {
        //         stash_index = Some(index);
        //         false // stop iteration
        //     } else {
        //         true // continue iteration
        //     }
        // })?;

        // if let Some(index) = stash_index {
        //     self.repo.stash_pop(index, None)?;
        //     self.stash_id = None;
        // }

        Ok(())
    }

    // pub fn reset_index(&mut self) -> Result<()> {
    //     let head = self.repo.head()?;
    //     let tree = head.peel_to_tree()?;
    //         .reset(&tree.into_object(), git2::ResetType::Mixed, None
    // }
}
