use std::path::PathBuf;

use git2::{Commit, Oid, Repository, StatusOptions, StatusShow, Tree};
use itertools::{Either, Itertools};
use miette::Result;
use miette::{Context, IntoDiagnostic, miette};

use crate::{env, glob};

pub struct Git {
    repo: Repository,
    stash: Option<Either<String, Oid>>,
    root: PathBuf,
}

impl Git {
    pub fn new() -> Result<Self> {
        let repo = Repository::open(".")
            .into_diagnostic()
            .wrap_err("failed to open repository")?;
        Ok(Self {
            root: repo.workdir().unwrap().to_path_buf(),
            repo,
            stash: None,
        })
    }

    fn head_tree(&self) -> Result<Tree<'_>> {
        let head = self
            .repo
            .head()
            .into_diagnostic()
            .wrap_err("failed to get head")?;
        let head = head
            .peel_to_tree()
            .into_diagnostic()
            .wrap_err("failed to peel head to tree")?;
        Ok(head)
    }

    fn head_commit(&self) -> Result<Commit<'_>> {
        let head = self
            .repo
            .head()
            .into_diagnostic()
            .wrap_err("failed to get head")?;
        let commit = head
            .peel_to_commit()
            .into_diagnostic()
            .wrap_err("failed to peel head to commit")?;
        Ok(commit)
    }

    fn head_commit_message(&self) -> Result<String> {
        let commit = self.head_commit()?;
        let message = commit
            .message()
            .ok_or(miette!("failed to get commit message"))?;
        Ok(message.to_string())
    }

    pub fn all_files(&self) -> Result<Vec<PathBuf>> {
        let head = self.head_tree()?;
        let mut files = Vec::new();
        head.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            if let Some(name) = entry.name() {
                let path = if root.is_empty() {
                    PathBuf::from(name)
                } else {
                    PathBuf::from(root).join(name)
                };
                if path.exists() {
                    files.push(path);
                }
            }
            git2::TreeWalkResult::Ok
        })
        .into_diagnostic()
        .wrap_err("failed to walk tree")?;
        Ok(files)
    }

    pub fn all_files_matching_globs(&self, globs: &[&str]) -> Result<Vec<PathBuf>> {
        let mut gb = globset::GlobSetBuilder::new();
        for g in globs {
            gb.add(globset::GlobBuilder::new(g).empty_alternates(true).build().into_diagnostic()?);
        }
        let gs = gb.build().into_diagnostic()?;
        let mut files = vec![];
        let head = self.head_tree()?;
        head.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            if let Some(name) = entry.name() {
                let path = if root.is_empty() {
                    PathBuf::from(name)
                } else {
                    PathBuf::from(root).join(name)
                };
                if path.exists() && gs.is_match(&path) {
                    files.push(path);
                }
            }
            // TODO: is there a way to prevent walking through everything and just paths that _could_ match a glob?
            git2::TreeWalkResult::Ok
        })
        .into_diagnostic()
        .wrap_err("failed to walk tree")?;
        Ok(files)
    }

    // pub fn intent_to_add_files(&self) -> Result<Vec<PathBuf>> {
    //     // let added_files = self.added_files()?;
    //     // TODO: get this to work, should be the equivalent of `git diff --name-only --diff-filter=A`
    //     Ok(vec![])
    // }

    pub fn staged_files(&self) -> Result<Vec<PathBuf>> {
        let mut status_options = StatusOptions::new();
        status_options.show(StatusShow::Index);
        let statuses = self
            .repo
            .statuses(Some(&mut status_options))
            .into_diagnostic()
            .wrap_err("failed to get statuses")?;
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
        let statuses = self
            .repo
            .statuses(Some(&mut status_options))
            .into_diagnostic()
            .wrap_err("failed to get statuses")?;
        let paths = statuses
            .iter()
            .filter_map(|s| s.path().map(PathBuf::from))
            .collect_vec();
        Ok(paths)
    }

    pub fn stash_unstaged(&mut self, force: bool) -> Result<()> {
        // Skip stashing if there's no initial commit yet or auto-stash is disabled
        if (!force && !*env::HK_STASH) || self.repo.head().is_err() {
            return Ok(());
        }

        // TODO: if any intent_to_add files exist, run `git rm --cached -- <file>...` then `git add --intent-to-add -- <file>...` when unstashing
        // let intent_to_add = self.intent_to_add_files()?;
        // see https://github.com/pre-commit/pre-commit/blob/main/pre_commit/staged_files_only.py
        if self.unstaged_files()?.is_empty() {
            return Ok(());
        }

        if let Ok(msg) = self.head_commit_message() {
            if msg.contains("Merge") {
                return Ok(());
            }
        }
        self.stash = if *env::HK_STASH_NO_GIT {
            self.build_diff()?.map(Either::Left)
        } else {
            self.push_stash()?.map(Either::Right)
        };
        if self.stash.is_none() {
            return Ok(());
        }

        debug!("removing unstaged files");

        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.allow_conflicts(true);
        checkout_opts.remove_untracked(true);
        checkout_opts.force();
        checkout_opts.update_index(false);
        self.repo
            .checkout_index(None, Some(&mut checkout_opts))
            .into_diagnostic()
            .wrap_err("failed to reset to head")?;

        Ok(())
    }

    fn build_diff(&self) -> Result<Option<String>> {
        debug!("building diff for stash");
        // essentially: git diff-index --ignore-submodules --binary --exit-code --no-color --no-ext-diff (git write-tree)
        let mut opts = git2::DiffOptions::new();
        opts.include_untracked(true);
        opts.show_binary(true);
        opts.show_untracked_content(true);
        let diff = self
            .repo
            .diff_index_to_workdir(None, Some(&mut opts))
            .into_diagnostic()
            .wrap_err("failed to get diff")?;
        let mut diff_bytes = vec![];
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            match line.origin() {
                '+' | '-' | ' ' => diff_bytes.push(line.origin() as u8),
                _ => {}
            };
            diff_bytes.extend(line.content());
            true
        })
        .into_diagnostic()
        .wrap_err("failed to print diff")?;
        let mut idx = self.repo.index().into_diagnostic()?;
        // if we can't write the index or there's no diff, don't stash
        if idx.write().is_err() || diff_bytes.is_empty() {
            Ok(None)
        } else {
            Ok(Some(String::from_utf8_lossy(&diff_bytes).to_string()))
        }
    }

    pub fn push_stash(&mut self) -> Result<Option<Oid>> {
        if self.unstaged_files()?.is_empty() {
            return Ok(None);
        }
        let sig = self.repo.signature().into_diagnostic()?;
        let mut flags = git2::StashFlags::default();
        flags.set(git2::StashFlags::INCLUDE_UNTRACKED, true);
        flags.set(git2::StashFlags::KEEP_INDEX, false);
        let oid = self
            .repo
            .stash_save2(&sig, None, Some(flags))
            .into_diagnostic()?;
        Ok(Some(oid))
    }

    pub fn pop_stash(&mut self) -> Result<()> {
        let Some(diff) = self.stash.take() else {
            return Ok(());
        };

        match diff {
            Either::Left(diff) => {
                let diff = git2::Diff::from_buffer(diff.as_bytes()).into_diagnostic()?;
                let mut apply_opts = git2::ApplyOptions::new();
                self.repo
                    .apply(&diff, git2::ApplyLocation::WorkDir, Some(&mut apply_opts))
                    .into_diagnostic()
                    .wrap_err("failed to apply diff")?;
            }
            Either::Right(_oid) => {
                let mut opts = git2::StashApplyOptions::new();
                let mut checkout_opts = git2::build::CheckoutBuilder::new();
                checkout_opts.allow_conflicts(true);
                checkout_opts.force();
                opts.checkout_options(checkout_opts);
                opts.reinstantiate_index();
                self.repo
                    .stash_pop(0, Some(&mut opts))
                    .into_diagnostic()
                    .wrap_err("failed to reset to stash")?;
            }
        }
        Ok(())
    }

    pub fn add(&self, pathspecs: &[&str]) -> Result<()> {
        let pathspecs = pathspecs
            .iter()
            .map(|p| p.replace(self.root.to_str().unwrap(), ""))
            .collect_vec();
        trace!("adding files: {:?}", &pathspecs);
        let mut index = self
            .repo
            .index()
            .into_diagnostic()
            .wrap_err("failed to get index")?;
        index
            .add_all(&pathspecs, git2::IndexAddOption::DEFAULT, None)
            .into_diagnostic()
            .wrap_err("failed to add files to index")?;
        index
            .write()
            .into_diagnostic()
            .wrap_err("failed to write index")?;
        Ok(())
    }
}
