use std::{
    cell::OnceCell,
    collections::BTreeSet,
    ffi::{CString, OsString},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::Result;
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use eyre::{WrapErr, eyre};
use git2::{Repository, StatusOptions, StatusShow};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use xx::file::display_path;

use crate::env;

fn parse_paths_from_patch(patch: &str) -> Vec<PathBuf> {
    // Parse lines like: diff --git a/path b/path
    let mut files = BTreeSet::new();
    for line in patch.lines() {
        if let Some(rest) = line.strip_prefix("diff --git a/") {
            if let Some((a_path, _b)) = rest.split_once(" b/") {
                let p = PathBuf::from(a_path);
                if p.exists() {
                    files.insert(p);
                }
            }
        }
    }
    files.into_iter().collect()
}

fn conflicted_files_from_porcelain(status_z: &str) -> Vec<PathBuf> {
    // In porcelain v1 short format: lines start with two status letters.
    // Unmerged statuses include: UU, AA, AU, UA, DU, UD
    let mut files = BTreeSet::new();
    for entry in status_z.split('\0').filter(|s| !s.is_empty()) {
        let mut chars = entry.chars();
        let x = chars.next().unwrap_or(' ');
        let y = chars.next().unwrap_or(' ');
        let path: String = chars.skip(1).collect();
        let is_unmerged = matches!(
            (x, y),
            ('U', 'U') | ('A', 'A') | ('A', 'U') | ('U', 'A') | ('D', 'U') | ('U', 'D')
        );
        if is_unmerged {
            let p = PathBuf::from(path);
            if p.exists() {
                files.insert(p);
            }
        }
    }
    files.into_iter().collect()
}

fn resolve_conflict_markers_preferring_theirs(path: &Path) -> Result<()> {
    let content = xx::file::read_to_string(path).unwrap_or_default();
    if !content.contains("<<<<<<<") || !content.contains(">>>>>>>") {
        return Ok(());
    }

    // Process as UTF-8 text, preserving original Unicode outside conflict blocks
    // and keeping exact line endings by using split_inclusive.
    let mut output = String::with_capacity(content.len());
    let parts: Vec<&str> = content.split_inclusive('\n').collect();
    let mut idx = 0usize;
    while idx < parts.len() {
        let line = parts[idx];
        if line.starts_with("<<<<<<<") {
            // Advance past our side until the separator '======='
            idx += 1;
            while idx < parts.len() && !parts[idx].starts_with("=======") {
                idx += 1;
            }
            // Skip the separator line if present
            if idx < parts.len() && parts[idx].starts_with("=======") {
                idx += 1;
            }
            // Capture the 'theirs' side until the closing '>>>>>>>'
            while idx < parts.len() && !parts[idx].starts_with(">>>>>>>") {
                output.push_str(parts[idx]);
                idx += 1;
            }
            // Skip the closing marker if present
            if idx < parts.len() && parts[idx].starts_with(">>>>>>>") {
                idx += 1;
            }
        } else {
            output.push_str(line);
            idx += 1;
        }
    }

    xx::file::write(path, output)?;
    Ok(())
}

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
            debug!("libgit2: true");
            let repo = Repository::open(".").wrap_err("failed to open repository")?;
            if let Some(index_file) = &*env::GIT_INDEX_FILE {
                // sets index to .git/index.lock which is used in the case of `git commit -a`
                let mut index = git2::Index::open(index_file).wrap_err("failed to get index")?;
                repo.set_index(&mut index)?;
            }
            Some(repo)
        } else {
            debug!("libgit2: false");
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

    pub fn all_files(&self, pathspec: Option<&[OsString]>) -> Result<BTreeSet<PathBuf>> {
        // TODO: handle pathspec to improve globbing
        if let Some(repo) = &self.repo {
            let idx = repo.index()?;
            Ok(idx
                .iter()
                .map(|i| {
                    let cstr = CString::new(&i.path[..]).unwrap();
                    #[cfg(unix)]
                    {
                        PathBuf::from(OsString::from_vec(cstr.as_bytes().to_vec()))
                    }
                    #[cfg(windows)]
                    {
                        PathBuf::from(cstr.into_string().unwrap())
                    }
                })
                .collect())
        } else {
            let mut cmd = xx::process::cmd("git", ["ls-files", "-z"]);
            if let Some(pathspec) = pathspec {
                cmd = cmd.arg("--");
                cmd = cmd.args(pathspec.iter().map(|p| p.to_str().unwrap()));
            }
            let output = cmd.read()?;
            Ok(output
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect())
        }
    }

    pub fn status(&self, pathspec: Option<&[OsString]>) -> Result<GitStatus> {
        if let Some(repo) = &self.repo {
            let mut status_options = StatusOptions::new();
            status_options.include_untracked(true);
            status_options.recurse_untracked_dirs(true);

            if let Some(pathspec) = pathspec {
                for path in pathspec {
                    status_options.pathspec(path);
                }
            }
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
            let mut args = vec![
                "status",
                "--porcelain",
                "--no-renames",
                "--untracked-files=all",
                "-z",
            ]
            .into_iter()
            .filter(|&arg| !arg.is_empty())
            .map(OsString::from)
            .collect_vec();
            if let Some(pathspec) = pathspec {
                args.push("--".into());
                args.extend(pathspec.iter().map(|p| p.into()))
            }
            let output = xx::process::cmd("git", args).read()?;
            let mut staged_files = BTreeSet::new();
            let mut unstaged_files = BTreeSet::new();
            let mut untracked_files = BTreeSet::new();
            let mut modified_files = BTreeSet::new();
            for file in output.split('\0') {
                let mut chars = file.chars();
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
                    let args = vec!["restore", "--worktree", "--"]
                        .into_iter()
                        .chain(status.modified_files.iter().map(|p| p.to_str().unwrap()))
                        .collect::<Vec<_>>();
                    xx::process::cmd("git", &args).run()?;
                }
                for file in status.untracked_files.iter() {
                    if let Err(err) = xx::file::remove_file(file) {
                        warn!("failed to remove untracked file: {err:?}");
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
            if *env::HK_STASH_UNTRACKED {
                opts.include_untracked(true);
            }
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
            if *env::HK_STASH_UNTRACKED && !status.untracked_files.is_empty() {
                let args = vec!["add", "--intent-to-add", "--"]
                    .into_iter()
                    .chain(status.unstaged_files.iter().map(|p| p.to_str().unwrap()))
                    .collect::<Vec<_>>();
                xx::process::cmd("git", &args).run()?;
            }
            let output =
                xx::process::sh("git diff --no-color --no-ext-diff --binary --ignore-submodules")?;
            if *env::HK_STASH_UNTRACKED && !status.untracked_files.is_empty() {
                let args = vec!["reset", "--"]
                    .into_iter()
                    .chain(status.unstaged_files.iter().map(|p| p.to_str().unwrap()))
                    .collect::<Vec<_>>();
                xx::process::cmd("git", &args).run()?;
            }
            output
        };
        let patch_file = self.patch_file();
        if let Err(err) = xx::file::write(patch_file, &patch) {
            warn!("failed to write patch file: {err:?}");
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
            if *env::HK_STASH_UNTRACKED {
                flags.set(git2::StashFlags::INCLUDE_UNTRACKED, true);
            }
            flags.set(git2::StashFlags::KEEP_INDEX, true);
            match repo.stash_save(&sig, "hk", Some(flags)) {
                Ok(_) => Ok(Some(StashType::LibGit)),
                Err(e) => {
                    // libgit2 sometimes fails with "attempt to merge diffs created with conflicting options"
                    // when there are both staged and unstaged changes. Fall back to shell git command.
                    debug!("libgit2 stash failed, falling back to shell git: {e}");
                    let mut cmd = xx::process::cmd("git", ["stash", "push", "--keep-index", "-m", "hk"]);
                    if *env::HK_STASH_UNTRACKED {
                        cmd = cmd.arg("--include-untracked");
                    }
                    cmd.run()?;
                    Ok(Some(StashType::Git))
                }
            }
        } else {
            let mut cmd = xx::process::cmd("git", ["stash", "push", "--keep-index", "-m", "hk"]);
            if *env::HK_STASH_UNTRACKED {
                cmd = cmd.arg("--include-untracked");
            }
            cmd.run()?;
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
                // Try a 3-way apply so non-conflicting hunks from the patch merge with
                // any fixer changes. Conflict markers will be written for conflicting hunks.
                let affected_files = parse_paths_from_patch(&diff);
                let apply_res = xx::process::cmd("git", ["apply", "--3way"])
                    .arg(&patch_file)
                    .run();
                // If 3-way apply reported an error, it might still have applied with conflicts.
                // Detect conflict markers in affected files and treat that as a successful apply
                // that requires resolution. Only fall back to plain apply when there are no
                // conflict markers present.
                let mut had_conflicts = false;
                for f in &affected_files {
                    if let Ok(s) = xx::file::read_to_string(f) {
                        if s.contains("<<<<<<<") && s.contains(">>>>>>>") {
                            had_conflicts = true;
                            break;
                        }
                    }
                }
                if let Err(err) = apply_res {
                    if had_conflicts {
                        warn!(
                            "git apply --3way returned error but left conflicts for {}: {err:?}; proceeding to resolve",
                            display_path(&patch_file)
                        );
                    } else {
                        warn!(
                            "git apply --3way failed for {}: {err:?}; attempting plain apply",
                            display_path(&patch_file)
                        );
                        if let Err(err2) = xx::process::cmd("git", ["apply"]).arg(&patch_file).run()
                        {
                            return Err(eyre!(
                                "failed to apply patch (3-way and plain) {}: {err:?}; {err2:?}",
                                display_path(&patch_file)
                            ));
                        }
                    }
                }
                // Resolve any conflict markers by preferring the patch (stash) side
                for f in affected_files {
                    if let Err(err) = resolve_conflict_markers_preferring_theirs(&f) {
                        warn!(
                            "failed to resolve conflict markers in {}: {err:?}",
                            display_path(&f)
                        );
                    }
                }
                if let Err(err) = xx::file::remove_file(patch_file) {
                    debug!("failed to remove patch file: {err:?}");
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
                // Apply the stash first, detect conflicts, and in case of conflict
                // prefer the stash (the original unstaged changes), discarding fixer edits.
                let apply_res = xx::process::cmd("git", ["stash", "apply"]).run();
                if let Err(err) = &apply_res {
                    warn!("git stash apply failed: {err:?}");
                }
                // Determine conflicted files via porcelain status
                let status = match xx::process::cmd(
                    "git",
                    [
                        "status",
                        "--porcelain",
                        "-z",
                        "--no-renames",
                        "--untracked-files=all",
                    ],
                )
                .read()
                {
                    Ok(s) => s,
                    Err(err) => {
                        warn!("failed to read git status: {err:?}");
                        String::new()
                    }
                };
                let conflicted = conflicted_files_from_porcelain(&status);
                if !conflicted.is_empty() {
                    // For each conflicted file, prefer the stash side only for conflicting hunks
                    for f in conflicted.iter() {
                        if let Err(err) = resolve_conflict_markers_preferring_theirs(f) {
                            warn!(
                                "failed to resolve conflict markers in {}: {err:?}",
                                display_path(f)
                            );
                        }
                        if let Err(err) = xx::process::cmd("git", ["add", "--"]).arg(f).run() {
                            warn!(
                                "failed to stage {} after resolving conflicts: {err:?}",
                                display_path(f)
                            );
                        }
                    }
                    // Drop the stash entry now that we've applied it
                    if let Err(err) = xx::process::cmd("git", ["stash", "drop"]).run() {
                        warn!("failed to drop stash after apply: {err:?}");
                    }
                } else if apply_res.is_err() {
                    // Last resort: try pop
                    if let Err(err) = xx::process::cmd("git", ["stash", "pop"]).run() {
                        warn!("git stash pop also failed after apply error: {err:?}");
                    }
                } else if let Err(err) = xx::process::cmd("git", ["stash", "drop"]).run() {
                    warn!("failed to drop stash after successful apply: {err:?}");
                }
            }
        }
        job.set_status(ProgressStatus::Done);
        Ok(())
    }

    pub fn add(&self, pathspecs: &[PathBuf]) -> Result<()> {
        let pathspecs = pathspecs.iter().collect_vec();
        trace!("adding files: {:?}", &pathspecs);
        if let Some(repo) = &self.repo {
            let mut index = repo.index().wrap_err("failed to get index")?;
            index
                .add_all(&pathspecs, git2::IndexAddOption::DEFAULT, None)
                .wrap_err("failed to add files to index")?;
            index.write().wrap_err("failed to write index")?;
            Ok(())
        } else {
            xx::process::cmd("git", ["add", "--"])
                .args(pathspecs)
                .stdout_capture()
                .run()?;
            Ok(())
        }
    }

    pub fn files_between_refs(&self, from_ref: &str, to_ref: Option<&str>) -> Result<Vec<PathBuf>> {
        let to_ref = to_ref.unwrap_or("HEAD");
        if let Some(repo) = &self.repo {
            let from_obj = repo
                .revparse_single(from_ref)
                .wrap_err(format!("Failed to parse reference: {from_ref}"))?;
            let to_obj = repo
                .revparse_single(to_ref)
                .wrap_err(format!("Failed to parse reference: {to_ref}"))?;

            // Find the merge base between the two references
            let merge_base = repo
                .merge_base(from_obj.id(), to_obj.id())
                .wrap_err("Failed to find merge base")?;
            let merge_base_obj = repo
                .find_object(merge_base, None)
                .wrap_err("Failed to find merge base object")?;
            let merge_base_tree = merge_base_obj
                .peel_to_tree()
                .wrap_err("Failed to get tree for merge base")?;

            let to_tree = to_obj
                .peel_to_tree()
                .wrap_err(format!("Failed to get tree for reference: {to_ref}"))?;

            let diff = repo
                .diff_tree_to_tree(Some(&merge_base_tree), Some(&to_tree), None)
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
            // Use git merge-base to find the common ancestor
            let merge_base = xx::process::sh(&format!("git merge-base {from_ref} {to_ref}"))?;
            let merge_base = merge_base.trim();

            let output = xx::process::cmd(
                "git",
                &[
                    "diff",
                    "-z",
                    "--name-only",
                    "--diff-filter=ACMRTUXB",
                    format!("{merge_base}..{to_ref}").as_str(),
                ],
            )
            .read()?;
            Ok(output
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect())
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GitStatus {
    pub unstaged_files: BTreeSet<PathBuf>,
    pub staged_files: BTreeSet<PathBuf>,
    pub untracked_files: BTreeSet<PathBuf>,
    pub modified_files: BTreeSet<PathBuf>,
}
