use std::{
    cell::OnceCell,
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::Result;
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use eyre::{WrapErr, eyre};
use git2::{Repository, StatusOptions, StatusShow, Tree};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use xx::file::display_path;

use crate::env;

pub struct Git {
    repo: Option<Repository>,
    stash: Option<StashType>,
    root: PathBuf,
    patch_file: OnceCell<PathBuf>,
}

enum StashType {
    PatchFile(String, PathBuf),
    LibGit,
    Git,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize, strum::EnumString)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum StashMethod {
    Git,
    PatchFile,
    None,
}

impl Git {
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let root = xx::file::find_up(&cwd, &[".git"])
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .ok_or(eyre!("failed to find git repository"))?;
        std::env::set_current_dir(&root)?;
        let repo = if *env::HK_LIBGIT2 {
            let repo = Repository::open(".").wrap_err("failed to open repository")?;
            if let Some(index_file) = &*env::GIT_INDEX_FILE {
                // sets index to .git/index.lock which is used in the case of `git commit -a`
                let mut index = git2::Index::open(index_file).wrap_err("failed to get index")?;
                repo.set_index(&mut index)?;
            }
            Some(repo)
        } else {
            None
        };
        Ok(Self {
            root,
            repo,
            stash: None,
            patch_file: OnceCell::new(),
        })
    }

    pub fn patch_file(&self) -> &Path {
        self.patch_file.get_or_init(|| {
            let name = self
                .root
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let rand = getrandom::u32()
                .unwrap_or_default()
                .to_string()
                .chars()
                .take(8)
                .collect::<String>();
            let date = chrono::Local::now().format("%Y-%m-%d").to_string();
            env::HK_STATE_DIR
                .join("patches")
                .join(format!("{name}-{date}-{rand}.patch"))
        })
    }

    pub fn matching_remote_branch(&self, remote: &str) -> Result<Option<String>> {
        if let Some(branch) = self.current_branch()? {
            if let Some(repo) = &self.repo {
                if let Ok(_ref) = repo.find_reference(&format!("refs/remotes/{remote}/{branch}")) {
                    return Ok(_ref.name().map(|s| s.to_string()));
                }
            } else {
                let output = xx::process::sh(&format!("git ls-remote --heads {remote} {branch}"))?;
                for line in output.lines() {
                    if line.contains(&format!("refs/remotes/{remote}/{branch}")) {
                        return Ok(Some(branch.to_string()));
                    }
                }
            }
        }
        Ok(None)
    }

    pub fn current_branch(&self) -> Result<Option<String>> {
        if let Some(repo) = &self.repo {
            let head = repo.head().wrap_err("failed to get head")?;
            let branch_name = head.shorthand().map(|s| s.to_string());
            Ok(branch_name)
        } else {
            let output = xx::process::sh("git branch --show-current")?;
            Ok(output.lines().next().map(|s| s.to_string()))
        }
    }

    fn head_tree(&self) -> Result<Tree<'_>> {
        let head = self
            .repo
            .as_ref()
            .unwrap()
            .head()
            .wrap_err("failed to get head")?;
        let head = head
            .peel_to_tree()
            .wrap_err("failed to peel head to tree")?;
        Ok(head)
    }

    // fn head_commit(&self) -> Result<Commit<'_>> {
    //     let head = self.repo.head().wrap_err("failed to get head")?;
    //     let commit = head
    //         .peel_to_commit()
    //         .wrap_err("failed to peel head to commit")?;
    //     Ok(commit)
    // }

    // fn head_commit_message(&self) -> Result<String> {
    //     let commit = self.head_commit()?;
    //     let message = commit
    //         .message()
    //         .ok_or(eyre!("failed to get commit message"))?;
    //     Ok(message.to_string())
    // }

    pub fn all_files(&self) -> Result<Vec<PathBuf>> {
        // TODO: should this also show untracked files?
        if let Some(_repo) = &self.repo {
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
            .wrap_err("failed to walk tree")?;
            Ok(files)
        } else {
            let output = xx::process::sh("git ls-files")?;
            Ok(output.lines().map(PathBuf::from).collect())
        }
    }

    pub fn status(&self) -> Result<GitStatus> {
        if let Some(repo) = &self.repo {
            let mut status_options = StatusOptions::new();
            status_options.include_untracked(true);

            // Get staged files
            status_options.show(StatusShow::Index);
            let staged_statuses = repo
                .statuses(Some(&mut status_options))
                .wrap_err("failed to get staged statuses")?;
            let staged_files = staged_statuses
                .iter()
                .filter_map(|s| s.path().map(PathBuf::from))
                .filter(|p| p.exists())
                .collect();

            // Get unstaged files
            status_options.show(StatusShow::Workdir);
            let unstaged_statuses = repo
                .statuses(Some(&mut status_options))
                .wrap_err("failed to get unstaged statuses")?;
            let unstaged_files = unstaged_statuses
                .iter()
                .filter_map(|s| s.path().map(PathBuf::from))
                .filter(|p| p.exists())
                .collect();
            let untracked_files = unstaged_statuses
                .iter()
                .filter(|s| s.status() == git2::Status::WT_NEW)
                .filter_map(|s| s.path().map(PathBuf::from))
                .collect();
            let modified_files = unstaged_statuses
                .iter()
                .filter(|s| {
                    s.status() == git2::Status::WT_MODIFIED
                        || s.status() == git2::Status::WT_TYPECHANGE
                })
                .filter_map(|s| s.path().map(PathBuf::from))
                .collect();

            Ok(GitStatus {
                staged_files,
                unstaged_files,
                untracked_files,
                modified_files,
            })
        } else {
            let output = xx::process::sh("git status --porcelain --no-renames")?;
            let mut staged_files = BTreeSet::new();
            let mut unstaged_files = BTreeSet::new();
            let mut untracked_files = BTreeSet::new();
            let mut modified_files = BTreeSet::new();
            for line in output.lines() {
                let mut chars = line.chars();
                let index_status = chars.next().unwrap_or_default();
                let workdir_status = chars.next().unwrap_or_default();
                let path = PathBuf::from(chars.skip(1).collect::<String>());
                let is_modified =
                    |c: char| c == 'M' || c == 'T' || c == 'A' || c == 'R' || c == 'C';
                if is_modified(index_status) {
                    staged_files.insert(path.clone());
                }
                if is_modified(workdir_status) || workdir_status == '?' {
                    unstaged_files.insert(path.clone());
                }
                if workdir_status == '?' {
                    untracked_files.insert(path.clone());
                }
                if is_modified(index_status) || is_modified(workdir_status) {
                    modified_files.insert(path);
                }
            }

            Ok(GitStatus {
                staged_files,
                unstaged_files,
                untracked_files,
                modified_files,
            })
        }
    }

    pub fn stash_unstaged(
        &mut self,
        job: &ProgressJob,
        method: StashMethod,
        status: &GitStatus,
    ) -> Result<()> {
        // Skip stashing if there's no initial commit yet or auto-stash is disabled
        if method == StashMethod::None {
            return Ok(());
        }
        if let Some(repo) = &self.repo {
            if repo.head().is_err() {
                return Ok(());
            }
        }
        job.set_body("{{spinner()}} stash – {{message}}{% if files is defined %} ({{files}} file{{files|pluralize}}){% endif %}");
        job.prop("message", "Fetching unstaged files");
        job.set_status(ProgressStatus::Running);

        job.prop("files", &status.unstaged_files.len());
        // TODO: if any intent_to_add files exist, run `git rm --cached -- <file>...` then `git add --intent-to-add -- <file>...` when unstashing
        // let intent_to_add = self.intent_to_add_files()?;
        // see https://github.com/pre-commit/pre-commit/blob/main/pre_commit/staged_files_only.py
        if status.unstaged_files.is_empty() {
            job.prop("message", "No unstaged changes to stash");
            job.set_status(ProgressStatus::Done);
            return Ok(());
        }

        // if let Ok(msg) = self.head_commit_message() {
        //     if msg.contains("Merge") {
        //         return Ok(());
        //     }
        // }
        self.stash = if method == StashMethod::PatchFile {
            job.prop(
                "message",
                &format!(
                    "Creating git diff patch – {}",
                    display_path(self.patch_file())
                ),
            );
            job.update();
            self.build_diff(status)?
        } else {
            job.prop("message", "Running git stash");
            job.update();
            self.push_stash(status)?
        };
        if self.stash.is_none() {
            job.prop("message", "No unstaged files to stash");
            job.set_status(ProgressStatus::Done);
            return Ok(());
        };

        job.prop("message", "Removing unstaged changes");
        job.update();

        if method == StashMethod::PatchFile {
            let patch_file = display_path(self.patch_file());
            job.prop(
                "message",
                &format!("Stashed unstaged changes in {patch_file}"),
            );
            if let Some(repo) = &self.repo {
                let mut checkout_opts = git2::build::CheckoutBuilder::new();
                checkout_opts.allow_conflicts(true);
                checkout_opts.remove_untracked(true);
                checkout_opts.force();
                checkout_opts.update_index(true);
                repo.checkout_index(None, Some(&mut checkout_opts))
                    .wrap_err("failed to reset to head")?;
            } else {
                if !status.modified_files.is_empty() {
                    let args = vec!["restore", "--staged", "--worktree", "--"]
                        .into_iter()
                        .chain(status.modified_files.iter().map(|p| p.to_str().unwrap()))
                        .collect::<Vec<_>>();
                    duct::cmd("git", &args).run()?;
                }
                for file in status.untracked_files.iter() {
                    if let Err(err) = xx::file::remove_file(file) {
                        warn!("failed to remove untracked file: {:?}", err);
                    }
                }
            }
        } else {
            job.prop("message", "Stashed unstaged changes with git stash");
        }
        job.set_status(ProgressStatus::Done);
        Ok(())
    }

    fn build_diff(&self, status: &GitStatus) -> Result<Option<StashType>> {
        debug!("building diff for stash");
        let patch = if let Some(repo) = &self.repo {
            // essentially: git diff-index --ignore-submodules --binary --exit-code --no-color --no-ext-diff (git write-tree)
            let mut opts = git2::DiffOptions::new();
            opts.include_untracked(true);
            opts.show_binary(true);
            opts.show_untracked_content(true);
            let diff = repo
                .diff_index_to_workdir(None, Some(&mut opts))
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
            .wrap_err("failed to print diff")?;
            let mut idx = repo.index()?;
            // if we can't write the index or there's no diff, don't stash
            if idx.write().is_err() || diff_bytes.is_empty() {
                return Ok(None);
            } else {
                String::from_utf8_lossy(&diff_bytes).to_string()
            }
        } else {
            if !status.untracked_files.is_empty() {
                let args = vec!["add", "--intent-to-add", "--"]
                    .into_iter()
                    .chain(status.unstaged_files.iter().map(|p| p.to_str().unwrap()))
                    .collect::<Vec<_>>();
                duct::cmd("git", &args).run()?;
            }
            let output =
                xx::process::sh("git diff --no-color --no-ext-diff --binary --ignore-submodules")?;
            if !status.untracked_files.is_empty() {
                let args = vec!["reset", "--"]
                    .into_iter()
                    .chain(status.unstaged_files.iter().map(|p| p.to_str().unwrap()))
                    .collect::<Vec<_>>();
                duct::cmd("git", &args).run()?;
            }
            output
        };
        let patch_file = self.patch_file();
        if let Err(err) = xx::file::write(patch_file, &patch) {
            warn!("failed to write patch file: {:?}", err);
        }
        Ok(Some(StashType::PatchFile(patch, patch_file.to_path_buf())))
    }

    fn push_stash(&mut self, status: &GitStatus) -> Result<Option<StashType>> {
        if status.unstaged_files.is_empty() {
            return Ok(None);
        }
        if let Some(repo) = &mut self.repo {
            let sig = repo.signature()?;
            let mut flags = git2::StashFlags::default();
            flags.set(git2::StashFlags::INCLUDE_UNTRACKED, true);
            flags.set(git2::StashFlags::KEEP_INDEX, true);
            repo.stash_save(&sig, "hk", Some(flags))
                .wrap_err("failed to stash")?;
            Ok(Some(StashType::LibGit))
        } else {
            xx::process::sh("git stash push --keep-index --include-untracked -m hk")?;
            Ok(Some(StashType::Git))
        }
    }

    pub fn pop_stash(&mut self) -> Result<()> {
        let Some(diff) = self.stash.take() else {
            return Ok(());
        };
        let job: Arc<ProgressJob>;

        match diff {
            StashType::PatchFile(diff, patch_file) => {
                job = ProgressJobBuilder::new()
                    .prop(
                        "message",
                        &format!(
                            "stash – Applying git diff patch – {}",
                            display_path(self.patch_file())
                        ),
                    )
                    .start();
                if let Some(repo) = &mut self.repo {
                    let diff = git2::Diff::from_buffer(diff.as_bytes())?;
                    let mut apply_opts = git2::ApplyOptions::new();
                    repo.apply(&diff, git2::ApplyLocation::WorkDir, Some(&mut apply_opts))
                        .wrap_err_with(|| {
                            format!("failed to apply diff {}", display_path(&patch_file))
                        })?;
                } else {
                    xx::process::sh(&format!("git apply {}", display_path(&patch_file)))?;
                }
                if let Err(err) = xx::file::remove_file(patch_file) {
                    debug!("failed to remove patch file: {:?}", err);
                }
            }
            // TODO: this does not work with untracked files
            // StashType::LibGit(_oid) => {
            //     job = ProgressJobBuilder::new()
            //         .prop("message", "stash – Applying git stash")
            //         .start();
            //         let repo =  self.repo.as_mut().unwrap();
            //         let mut opts = git2::StashApplyOptions::new();
            //         let mut checkout_opts = git2::build::CheckoutBuilder::new();
            //         checkout_opts.allow_conflicts(true);
            //         checkout_opts.update_index(true);
            //         checkout_opts.force();
            //         opts.checkout_options(checkout_opts);
            //         opts.reinstantiate_index();
            //         repo.stash_pop(0, Some(&mut opts))
            //         .wrap_err("failed to pop stash")?;
            // }
            StashType::LibGit | StashType::Git => {
                job = ProgressJobBuilder::new()
                    .prop("message", "stash – Applying git stash")
                    .start();
                xx::process::sh("git stash pop")?;
            }
        }
        job.set_status(ProgressStatus::Done);
        Ok(())
    }

    pub fn add(&self, pathspecs: &[String]) -> Result<()> {
        let pathspecs = pathspecs
            .iter()
            .map(|p| p.replace(self.root.to_str().unwrap(), ""))
            .collect_vec();
        trace!("adding files: {:?}", &pathspecs);
        if let Some(repo) = &self.repo {
            let mut index = repo.index().wrap_err("failed to get index")?;
            index
                .add_all(&pathspecs, git2::IndexAddOption::DEFAULT, None)
                .wrap_err("failed to add files to index")?;
            index.write().wrap_err("failed to write index")?;
            Ok(())
        } else {
            xx::process::sh(&format!("git add {}", pathspecs.join(" ")))?;
            Ok(())
        }
    }

    pub fn files_between_refs(&self, from_ref: &str, to_ref: &str) -> Result<Vec<PathBuf>> {
        if let Some(repo) = &self.repo {
            let from_obj = repo
                .revparse_single(from_ref)
                .wrap_err(format!("Failed to parse reference: {}", from_ref))?;
            let to_obj = repo
                .revparse_single(to_ref)
                .wrap_err(format!("Failed to parse reference: {}", to_ref))?;

            let from_tree = from_obj
                .peel_to_tree()
                .wrap_err(format!("Failed to get tree for reference: {}", from_ref))?;
            let to_tree = to_obj
                .peel_to_tree()
                .wrap_err(format!("Failed to get tree for reference: {}", to_ref))?;

            let diff = repo
                .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)
                .wrap_err("Failed to get diff between references")?;

            let mut files = BTreeSet::new();
            diff.foreach(
                &mut |_, _| true,
                None,
                None,
                Some(&mut |diff_delta, _, _| {
                    if let Some(path) = diff_delta.new_file().path() {
                        let path_buf = PathBuf::from(path);
                        if path_buf.exists() {
                            files.insert(path_buf);
                        }
                    }
                    true
                }),
            )
            .wrap_err("Failed to process diff")?;

            Ok(files.into_iter().collect())
        } else {
            let output = xx::process::sh(&format!(
                "git diff --name-only --diff-filter=ACRMT {}..{}",
                from_ref, to_ref
            ))?;
            Ok(output.lines().map(PathBuf::from).collect())
        }
    }
}

#[derive(Debug)]
pub(crate) struct GitStatus {
    pub unstaged_files: BTreeSet<PathBuf>,
    pub staged_files: BTreeSet<PathBuf>,
    pub untracked_files: BTreeSet<PathBuf>,
    pub modified_files: BTreeSet<PathBuf>,
}
