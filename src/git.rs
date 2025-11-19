use std::{
    collections::BTreeSet,
    ffi::{CString, OsString},
    path::PathBuf,
};

use crate::Result;
use crate::merge;
use crate::settings::Settings;
use crate::ui::style;
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use eyre::{WrapErr, eyre};
use git2::{Repository, StatusOptions, StatusShow};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use xx::file::display_path;

use crate::env;

fn git_cmd<I, S>(args: I) -> xx::process::XXExpression
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args.into_iter().map(|s| s.into()).collect::<Vec<_>>();
    xx::process::cmd("git", args).on_stderr_line(|line| {
        clx::progress::with_terminal_lock(|| eprintln!("{} {}", style::edim("git"), line))
    })
}

fn git_cmd_silent<I, S>(args: I) -> xx::process::XXExpression
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args = args.into_iter().map(|s| s.into()).collect::<Vec<_>>();
    // Silently ignore stderr output by using an empty handler
    xx::process::cmd("git", args).on_stderr_line(|_line| {})
}

fn git_run<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    git_cmd(args)
        .on_stdout_line(|line| {
            clx::progress::with_terminal_lock(|| eprintln!("{} {}", style::edim("git"), line))
        })
        .run()?;
    Ok(())
}

fn git_read<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    Ok(git_cmd(args).read()?)
}

fn git_read_raw<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    // Don't use git_cmd here because it adds stderr handlers which breaks stdout capture
    let args = args.into_iter().map(|s| s.into()).collect::<Vec<_>>();
    let output = xx::process::cmd("git", args).stdout_capture().run()?;
    String::from_utf8(output.stdout).map_err(|err| eyre!("git output is not valid UTF-8: {err}"))
}

pub struct Git {
    repo: Option<Repository>,
    stash: Option<StashType>,
    // Commit id of the stash entry we created (top-of-stack at creation time)
    stash_commit: Option<String>,
    saved_index: Option<Vec<(u32, String, PathBuf)>>,
    saved_worktree: Option<std::collections::HashMap<PathBuf, String>>,
}

enum StashType {
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
            repo,
            stash: None,
            stash_commit: None,
            saved_index: None,
            saved_worktree: None,
        })
    }

    /// Get the patches directory for this repository
    fn patches_dir(&self) -> Result<PathBuf> {
        let patches_dir = env::HK_STATE_DIR.join("patches");
        std::fs::create_dir_all(&patches_dir)?;
        Ok(patches_dir)
    }

    /// Get a unique name for the repository based on its directory
    fn repo_name(&self) -> Result<String> {
        let cwd = std::env::current_dir()?;
        let name = cwd
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        Ok(name)
    }

    /// Rotate patch files, keeping only the last N patches for this repository
    fn rotate_patch_files(&self, keep_count: usize) -> Result<()> {
        let patches_dir = self.patches_dir()?;
        let repo_name = self.repo_name()?;
        let prefix = format!("{}-", repo_name);

        // Collect all patch files for this repo
        let mut patch_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&patches_dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.starts_with(&prefix) && file_name.ends_with(".patch") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                patch_files.push((entry.path(), modified));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modification time, newest first
        patch_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Remove old patches beyond keep_count
        for (path, _) in patch_files.iter().skip(keep_count) {
            debug!("Rotating old patch file: {}", path.display());
            let _ = std::fs::remove_file(path);
        }

        Ok(())
    }

    /// Save a patch backup of the stash
    fn save_stash_patch(&self, stash_ref: &str) {
        // If backup_count is 0, skip patch backup entirely
        let backup_count = Settings::get().stash_backup_count;
        if backup_count == 0 {
            return;
        }

        // Get patches directory and repo name
        let (patches_dir, repo_name) = match (self.patches_dir(), self.repo_name()) {
            (Ok(dir), Ok(name)) => (dir, name),
            (Err(e), _) => {
                warn!("Failed to get patches directory: {}", e);
                return;
            }
            (_, Err(e)) => {
                warn!("Failed to get repository name: {}", e);
                return;
            }
        };

        // Generate timestamp and short hash for unique filename
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let short_hash = if let Some(commit) = &self.stash_commit {
            commit.chars().take(8).collect::<String>()
        } else {
            "unknown".to_string()
        };

        let patch_filename = format!("{}-{}-{}.patch", repo_name, timestamp, short_hash);
        let patch_path = patches_dir.join(&patch_filename);

        // Generate patch using git stash show
        let mut cmd = git_cmd_silent(["stash", "show", "-p"]);
        if *env::HK_STASH_UNTRACKED {
            cmd = cmd.arg("--include-untracked");
        }
        cmd = cmd.arg(stash_ref);

        // Read patch content from git
        let patch_content = match cmd.read() {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to generate stash patch: {}", e);
                return;
            }
        };

        // Write patch file
        if let Err(e) = std::fs::write(&patch_path, patch_content) {
            warn!(
                "Failed to write stash patch to {}: {}",
                patch_path.display(),
                e
            );
            return;
        }
        debug!("Saved stash patch: {}", patch_path.display());

        // Rotate old patches based on configured backup count
        if let Err(e) = self.rotate_patch_files(backup_count) {
            warn!("Failed to rotate old patch files: {}", e);
            // Continue anyway - at least we saved the current patch
        }
    }

    /// Determine the repository's default branch reference.
    /// Strategy:
    /// 1) Use `origin/HEAD` if it points to a branch
    /// 2) If current branch exists on origin, use that
    /// 3) Otherwise, try `main` then `master` if they exist
    pub fn default_branch(&self) -> Result<String> {
        // Try origin/HEAD -> refs/remotes/origin/HEAD -> symbolic-ref
        // Shell git path (works with or without libgit2 enabled)
        let head_sym = git_cmd(["symbolic-ref", "refs/remotes/origin/HEAD"]).read();
        if let Ok(symref) = head_sym {
            if let Some(target) = symref.lines().next() {
                // Expect something like: refs/remotes/main
                if let Some(short) = target.strip_prefix("refs/remotes/") {
                    return Ok(short.to_string());
                }
            }
        }

        // If current branch has a remote counterpart, prefer it
        if let Ok(Some(rb)) = self.matching_remote_branch("origin") {
            if let Some(short) = rb.strip_prefix("refs/remotes/") {
                return Ok(short.to_string());
            }
            return Ok(rb);
        }

        // Fallbacks: main, master
        for cand in ["main", "master"] {
            let branch = cand.split('/').next_back().unwrap();
            let out = xx::process::sh(&format!("git ls-remote --heads origin {}", branch))?;
            if out
                .lines()
                .any(|l| l.ends_with(&format!("refs/heads/{}", branch)))
            {
                return Ok(cand.to_string());
            }
        }

        // As a last resort, return origin/HEAD literal to let callers handle errors
        Ok("origin/HEAD".to_string())
    }

    /// Resolve the effective default branch, honoring a configured override in project config.
    /// If `Config.default_branch` is set and non-empty, it is returned as-is. Otherwise, falls back to detection.
    pub fn resolve_default_branch(&self) -> String {
        if let Ok(cfg) = crate::config::Config::get() {
            if let Some(val) = cfg.default_branch {
                if !val.trim().is_empty() {
                    return val;
                }
            }
        }
        self.default_branch().unwrap_or_else(|_| "main".to_string())
    }
    // removed: patch_file path helper

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
            let mut cmd = git_cmd(["ls-files", "-z"]);
            if let Some(pathspec) = pathspec {
                cmd = cmd.arg("--");
                cmd = cmd.args(pathspec.iter().filter_map(|p| p.to_str()));
            }
            let output = cmd.read()?;
            Ok(output
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect())
        }
    }

    #[tracing::instrument(level = "info", name = "git.status", skip(self, pathspec), fields(pathspec_count = pathspec.as_ref().map(|p| p.len()).unwrap_or(0)))]
    pub fn status(&self, pathspec: Option<&[OsString]>) -> Result<GitStatus> {
        // Refresh index stat information to avoid stale mtime/size causing mis-detection
        let _ = git_run(["update-index", "-q", "--refresh"]);
        if let Some(repo) = &self.repo {
            let mut status_options = StatusOptions::new();
            status_options.include_untracked(true);
            status_options.recurse_untracked_dirs(true);
            status_options.renames_head_to_index(true);

            if let Some(pathspec) = pathspec {
                for path in pathspec {
                    status_options.pathspec(path);
                }
            }
            // Get staged files (index)
            status_options.show(StatusShow::Index);
            let staged_statuses = repo
                .statuses(Some(&mut status_options))
                .wrap_err("failed to get staged statuses")?;
            let mut staged_files = BTreeSet::new();
            let mut staged_added_files = BTreeSet::new();
            let mut staged_modified_files = BTreeSet::new();
            let mut staged_deleted_files = BTreeSet::new();
            let mut staged_renamed_files = BTreeSet::new();
            let staged_copied_files = BTreeSet::new();
            for s in staged_statuses.iter() {
                if let Some(path) = s.path().map(PathBuf::from) {
                    // Check if path exists (including broken symlinks)
                    // path.exists() returns false for broken symlinks, but symlink_metadata succeeds
                    let exists = path.exists() || std::fs::symlink_metadata(&path).is_ok();
                    let st = s.status();
                    if st.is_index_new() {
                        staged_added_files.insert(path.clone());
                    }
                    if st.is_index_modified() || st.is_index_typechange() {
                        staged_modified_files.insert(path.clone());
                    }
                    if st.is_index_deleted() {
                        staged_deleted_files.insert(path.clone());
                    }
                    if st.is_index_renamed() {
                        staged_renamed_files.insert(path.clone());
                    }
                    // libgit2 does not expose an index-copied accessor; keep empty here
                    if exists {
                        staged_files.insert(path);
                    }
                }
            }

            // Get unstaged files (workdir)
            status_options.show(StatusShow::Workdir);
            let unstaged_statuses = repo
                .statuses(Some(&mut status_options))
                .wrap_err("failed to get unstaged statuses")?;
            let mut unstaged_files = BTreeSet::new();
            let mut untracked_files = BTreeSet::new();
            let mut modified_files = BTreeSet::new();
            let mut unstaged_modified_files = BTreeSet::new();
            let mut unstaged_deleted_files = BTreeSet::new();
            let mut unstaged_renamed_files = BTreeSet::new();
            for s in unstaged_statuses.iter() {
                if let Some(path) = s.path().map(PathBuf::from) {
                    // Check if path exists (including broken symlinks)
                    // path.exists() returns false for broken symlinks, but symlink_metadata succeeds
                    let exists = path.exists() || std::fs::symlink_metadata(&path).is_ok();
                    let st = s.status();
                    if st == git2::Status::WT_NEW {
                        untracked_files.insert(path.clone());
                    }
                    if st == git2::Status::WT_MODIFIED || st == git2::Status::WT_TYPECHANGE {
                        modified_files.insert(path.clone());
                        unstaged_modified_files.insert(path.clone());
                    }
                    if st == git2::Status::WT_DELETED {
                        unstaged_deleted_files.insert(path.clone());
                    }
                    if st == git2::Status::WT_RENAMED {
                        unstaged_renamed_files.insert(path.clone());
                    }
                    if exists {
                        unstaged_files.insert(path);
                    }
                }
            }

            Ok(GitStatus {
                staged_files,
                unstaged_files,
                untracked_files,
                modified_files,
                staged_added_files,
                staged_modified_files,
                staged_deleted_files,
                staged_renamed_files,
                staged_copied_files,
                unstaged_modified_files,
                unstaged_deleted_files,
                unstaged_renamed_files,
            })
        } else {
            let mut args = vec!["status", "--porcelain", "--untracked-files=all", "-z"]
                .into_iter()
                .filter(|&arg| !arg.is_empty())
                .map(OsString::from)
                .collect_vec();
            if let Some(pathspec) = pathspec {
                args.push("--".into());
                args.extend(pathspec.iter().map(|p| p.into()))
            }
            let output = git_read(args)?;
            let mut staged_files = BTreeSet::new();
            let mut unstaged_files = BTreeSet::new();
            let mut untracked_files = BTreeSet::new();
            let mut modified_files = BTreeSet::new();
            let mut staged_added_files = BTreeSet::new();
            let mut staged_modified_files = BTreeSet::new();
            let mut staged_deleted_files = BTreeSet::new();
            let mut staged_renamed_files = BTreeSet::new();
            let mut staged_copied_files = BTreeSet::new();
            let mut unstaged_modified_files = BTreeSet::new();
            let mut unstaged_deleted_files = BTreeSet::new();
            let mut unstaged_renamed_files = BTreeSet::new();
            for file in output.split('\0') {
                if file.is_empty() {
                    continue;
                }
                let mut chars = file.chars();
                let index_status = chars.next().unwrap_or_default();
                let workdir_status = chars.next().unwrap_or_default();
                let path = PathBuf::from(chars.skip(1).collect::<String>());
                // Check if path exists (including broken symlinks)
                // path.exists() returns false for broken symlinks, but symlink_metadata succeeds
                let exists = path.exists() || std::fs::symlink_metadata(&path).is_ok();
                let is_modified =
                    |c: char| c == 'M' || c == 'T' || c == 'A' || c == 'R' || c == 'C';

                // Only consider staged files that still exist in the worktree to avoid AD cases
                if is_modified(index_status) && workdir_status != 'D' && exists {
                    staged_files.insert(path.clone());
                }
                // Classify staged/index status
                match index_status {
                    'A' => {
                        staged_added_files.insert(path.clone());
                    }
                    'M' | 'T' => {
                        staged_modified_files.insert(path.clone());
                    }
                    'D' => {
                        staged_deleted_files.insert(path.clone());
                    }
                    'R' => {
                        staged_renamed_files.insert(path.clone());
                    }
                    'C' => {
                        staged_copied_files.insert(path.clone());
                    }
                    _ => {}
                }
                // Unstaged files include actual worktree changes and untracked files
                if (is_modified(workdir_status) || workdir_status == '?') && exists {
                    unstaged_files.insert(path.clone());
                }
                if workdir_status == '?' && exists {
                    untracked_files.insert(path.clone());
                }
                // Track modified files only if the path exists
                if (is_modified(index_status) || is_modified(workdir_status)) && exists {
                    modified_files.insert(path.clone());
                }
                // Classify workdir status
                match workdir_status {
                    'M' | 'T' => {
                        unstaged_modified_files.insert(path.clone());
                    }
                    'D' => {
                        unstaged_deleted_files.insert(path.clone());
                    }
                    'R' => {
                        unstaged_renamed_files.insert(path.clone());
                    }
                    _ => {}
                }
            }

            Ok(GitStatus {
                staged_files,
                unstaged_files,
                untracked_files,
                modified_files,
                staged_added_files,
                staged_modified_files,
                staged_deleted_files,
                staged_renamed_files,
                staged_copied_files,
                unstaged_modified_files,
                unstaged_deleted_files,
                unstaged_renamed_files,
            })
        }
    }

    #[tracing::instrument(level = "info", name = "git.stash.push", skip_all)]
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

        // Hardened detection of worktree-only changes (including partially staged files)
        let mut files_to_stash: BTreeSet<PathBuf> = BTreeSet::new();
        // 1) git diff --name-only (worktree vs index)
        {
            let args: Vec<OsString> = vec![
                "diff".into(),
                "--name-only".into(),
                "-z".into(),
                "--no-ext-diff".into(),
                "--ignore-submodules".into(),
            ];
            let out = git_read(args).unwrap_or_default();
            for name in out.split('\0') {
                if name.is_empty() {
                    continue;
                }
                let p = PathBuf::from(name);
                if p.exists() {
                    files_to_stash.insert(p);
                }
            }
        }
        // 2) git ls-files -m (modified in worktree)
        {
            let args: Vec<OsString> = vec!["ls-files".into(), "-m".into(), "-z".into()];
            let out = git_read(args).unwrap_or_default();
            for name in out.split('\0') {
                if name.is_empty() {
                    continue;
                }
                let p = PathBuf::from(name);
                if p.exists() {
                    files_to_stash.insert(p);
                }
            }
        }
        // 3) Parse porcelain to catch nuanced mixed states
        {
            let args: Vec<OsString> = vec![
                "status".into(),
                "--porcelain".into(),
                "--no-renames".into(),
                "--untracked-files=all".into(),
                "-z".into(),
            ];
            let out = git_read(args).unwrap_or_default();
            for entry in out.split('\0').filter(|s| !s.is_empty()) {
                let mut chars = entry.chars();
                let _x = chars.next().unwrap_or_default();
                let y = chars.next().unwrap_or_default();
                let path = chars.skip(1).collect::<String>();
                if y == 'M' || y == 'T' || y == 'R' {
                    // worktree side has changes
                    let p = PathBuf::from(&path);
                    if p.exists() {
                        files_to_stash.insert(p);
                    }
                }
            }
        }
        // 4) Union with computed status for safety
        for p in status.unstaged_files.iter() {
            files_to_stash.insert(p.clone());
        }
        // 5) When HK_STASH_UNTRACKED=true, also include untracked files
        if *env::HK_STASH_UNTRACKED {
            for p in status.untracked_files.iter() {
                files_to_stash.insert(p.clone());
            }
        }
        let files_count = files_to_stash.len();
        job.prop("files", &files_count);
        // TODO: if any intent_to_add files exist, run `git rm --cached -- <file>...` then `git add --intent-to-add -- <file>...` when unstashing
        // let intent_to_add = self.intent_to_add_files()?;
        // see https://github.com/pre-commit/pre-commit/blob/main/pre_commit/staged_files_only.py
        if files_to_stash.is_empty() {
            job.prop("message", "No unstaged changes to stash");
            job.set_status(ProgressStatus::Done);
            return Ok(());
        }

        // if let Ok(msg) = self.head_commit_message() {
        //     if msg.contains("Merge") {
        //         return Ok(());
        //     }
        // }
        job.prop("message", "Running git stash");
        job.update();
        let subset_vec: Vec<PathBuf> = files_to_stash.iter().cloned().collect();
        let subset_opt: Option<&[PathBuf]> = if subset_vec.is_empty() {
            None
        } else {
            Some(&subset_vec[..])
        };
        self.stash = self.push_stash(subset_opt, status)?;
        if self.stash.is_none() {
            job.prop("message", "No unstaged files to stash");
            job.set_status(ProgressStatus::Done);
            return Ok(());
        };

        job.prop("message", "Removing unstaged changes");
        job.update();

        job.prop("message", "Stashed unstaged changes");
        job.set_status(ProgressStatus::Done);
        Ok(())
    }

    // removed: build_diff helper

    // removed patch-file custom path for now

    fn push_stash(
        &mut self,
        paths: Option<&[PathBuf]>,
        status: &GitStatus,
    ) -> Result<Option<StashType>> {
        // When a subset of paths is provided, filter out untracked paths. Passing untracked
        // paths as pathspecs to `git stash push` can fail with "did not match any file(s) known to git".
        // The --include-untracked flag will automatically handle all untracked files.
        let tracked_subset: Option<Vec<PathBuf>> = paths.map(|ps| {
            ps.iter()
                .filter(|p| !status.untracked_files.contains(*p))
                .cloned()
                .collect()
        });
        // If after filtering there are no tracked paths left:
        // - When HK_STASH_UNTRACKED=true, do a full stash (no pathspecs) to stash all untracked files
        // - Otherwise, no need to stash anything
        if let Some(ref ts) = tracked_subset {
            if ts.is_empty() {
                if *env::HK_STASH_UNTRACKED {
                    // No tracked files to stash, but we want to stash all untracked files
                    // So do a full stash with --include-untracked (no pathspecs)
                    return self.push_stash(None, status);
                } else {
                    return Ok(None);
                }
            }
        }
        if let Some(repo) = &mut self.repo {
            let sig = repo.signature()?;
            let mut flags = git2::StashFlags::default();
            if *env::HK_STASH_UNTRACKED {
                flags.set(git2::StashFlags::INCLUDE_UNTRACKED, true);
            }
            flags.set(git2::StashFlags::KEEP_INDEX, true);
            // If partial paths requested, force shell git path since libgit2 does not support it
            if let Some(paths) = tracked_subset.as_deref() {
                let mut cmd = git_cmd(["stash", "push", "--keep-index", "-m", "hk"]);
                if *env::HK_STASH_UNTRACKED {
                    cmd = cmd.arg("--include-untracked");
                }
                let utf8_paths: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
                if !utf8_paths.is_empty() {
                    cmd = cmd.arg("--");
                    cmd = cmd.args(utf8_paths);
                }
                cmd.run()?;
                // Record the stash commit we just created and save patch backup
                if let Ok(h) = git_cmd(["rev-parse", "-q", "--verify", "stash@{0}"]).read() {
                    let commit_hash = h.trim().to_string();
                    self.stash_commit = Some(commit_hash.clone());
                    self.save_stash_patch(&commit_hash);
                }
                Ok(Some(StashType::Git))
            } else {
                match repo.stash_save(&sig, "hk", Some(flags)) {
                    Ok(_) => {
                        // Record the stash commit we just created and save patch backup
                        if let Ok(h) = git_cmd(["rev-parse", "-q", "--verify", "stash@{0}"]).read()
                        {
                            let commit_hash = h.trim().to_string();
                            self.stash_commit = Some(commit_hash.clone());
                            self.save_stash_patch(&commit_hash);
                        }
                        Ok(Some(StashType::LibGit))
                    }
                    Err(e) => {
                        debug!("libgit2 stash failed, falling back to shell git: {e}");
                        let mut cmd = git_cmd(["stash", "push", "--keep-index", "-m", "hk"]);
                        if *env::HK_STASH_UNTRACKED {
                            cmd = cmd.arg("--include-untracked");
                        }
                        cmd.run()?;
                        // Record the stash commit we just created and save patch backup
                        if let Ok(h) = git_cmd(["rev-parse", "-q", "--verify", "stash@{0}"]).read()
                        {
                            let commit_hash = h.trim().to_string();
                            self.stash_commit = Some(commit_hash.clone());
                            self.save_stash_patch(&commit_hash);
                        }
                        Ok(Some(StashType::Git))
                    }
                }
            }
        } else {
            let mut cmd = git_cmd(["stash", "push", "--keep-index", "-m", "hk"]);
            if *env::HK_STASH_UNTRACKED {
                cmd = cmd.arg("--include-untracked");
            }
            if let Some(paths) = tracked_subset.as_deref() {
                let utf8_paths: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
                if !utf8_paths.is_empty() {
                    cmd = cmd.arg("--");
                    cmd = cmd.args(utf8_paths);
                }
            }
            cmd.run()?;
            // Record the stash commit we just created and save patch backup
            if let Ok(h) = git_cmd(["rev-parse", "-q", "--verify", "stash@{0}"]).read() {
                let commit_hash = h.trim().to_string();
                self.stash_commit = Some(commit_hash.clone());
                self.save_stash_patch(&commit_hash);
            }
            Ok(Some(StashType::Git))
        }
    }

    // removed: push_stash_keep_index_no_untracked helper

    pub fn capture_index(&mut self, paths: &[PathBuf]) -> Result<()> {
        if paths.is_empty() {
            self.saved_index = Some(vec![]);
            self.saved_worktree = Some(std::collections::HashMap::new());
            return Ok(());
        }
        let mut args: Vec<OsString> = vec!["ls-files".into(), "-s".into(), "-z".into()];
        args.push("--".into());
        args.extend(paths.iter().map(|p| OsString::from(p.as_os_str())));
        let out = git_read(args)?;
        let mut entries: Vec<(u32, String, PathBuf)> = vec![];
        let mut wt_map: std::collections::HashMap<PathBuf, String> =
            std::collections::HashMap::new();
        for rec in out.split('\0').filter(|s| !s.is_empty()) {
            // format: mode SP oid SP stage TAB path
            // example: 100644 0123456789abcdef... 0	path/to/file
            if let Some((left, path)) = rec.split_once('\t') {
                let mut parts = left.split_whitespace();
                let mode = parts.next().unwrap_or("100644");
                let oid = parts.next().unwrap_or("");
                if !oid.is_empty() {
                    let mode = u32::from_str_radix(mode, 8).unwrap_or(0o100644);
                    let p = PathBuf::from(path);
                    entries.push((mode, oid.to_string(), p.clone()));
                    // Capture current worktree contents to preserve exact EOF newline state
                    if p.exists() {
                        if let Ok(contents) = xx::file::read_to_string(&p) {
                            wt_map.insert(p.clone(), contents);
                        }
                    }
                }
            }
        }
        self.saved_index = Some(entries);
        self.saved_worktree = Some(wt_map);
        Ok(())
    }

    pub fn pop_stash(&mut self) -> Result<()> {
        let Some(diff) = self.stash.take() else {
            return Ok(());
        };
        let job = ProgressJobBuilder::new()
            .prop("message", "stash – Restoring unstaged changes (manual)")
            .start();
        match diff {
            StashType::LibGit | StashType::Git => {
                // Resolve the specific stash entry we created using its commit id, falling back to top
                let stash_ref = if let Some(hash) = self.stash_commit.as_ref() {
                    let list = git_cmd(["stash", "list", "--format=%H %gd"])
                        .read()
                        .unwrap_or_default();
                    let mut found: Option<String> = None;
                    for line in list.lines() {
                        let mut parts = line.split_whitespace();
                        if let (Some(h), Some(gd)) = (parts.next(), parts.next()) {
                            if h == hash {
                                found = Some(gd.to_string());
                                break;
                            }
                        }
                    }
                    found.unwrap_or_else(|| "stash@{0}".to_string())
                } else {
                    "stash@{0}".to_string()
                };

                // List paths from our stash entry
                // When HK_STASH_UNTRACKED=true, we need to include untracked files in the show output
                let mut cmd = git_cmd(["stash", "show", "--name-only", "-z"]);
                if *env::HK_STASH_UNTRACKED {
                    cmd = cmd.arg("--include-untracked");
                }
                cmd = cmd.arg(&stash_ref);
                let show = cmd.read().unwrap_or_default();
                let stash_paths: Vec<PathBuf> = show
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .collect();

                // Build a map of CURRENT index (post-step) entries to re-stage Fixer blobs.
                // Only include files that are actually staged-changed to avoid treating unrelated
                // tracked files (e.g., lockfiles) as fixers and pulling their contents into memory.
                let mut fixer_map: std::collections::HashMap<PathBuf, (u32, String)> =
                    std::collections::HashMap::new();
                // Determine the set of paths with staged changes (index differs from HEAD)
                let staged_changed_set: std::collections::HashSet<PathBuf> =
                    git_cmd(["diff", "--name-only", "--cached", "-z"])
                        .read()
                        .unwrap_or_default()
                        .split('\0')
                        .filter(|s| !s.is_empty())
                        .map(PathBuf::from)
                        .collect();
                if !stash_paths.is_empty() {
                    let mut args: Vec<OsString> =
                        vec!["ls-files".into(), "-s".into(), "-z".into(), "--".into()];
                    args.extend(
                        stash_paths
                            .iter()
                            .filter_map(|p| p.to_str())
                            .map(OsString::from),
                    );
                    if let Ok(list) = git_read(args) {
                        for rec in list.split('\0').filter(|s| !s.is_empty()) {
                            // format: mode SP oid SP stage TAB path
                            if let Some((left, path)) = rec.split_once('\t') {
                                let mut parts = left.split_whitespace();
                                let mode = parts.next().unwrap_or("100644");
                                let oid = parts.next().unwrap_or("");
                                let path_buf = PathBuf::from(path);
                                if !oid.is_empty() && staged_changed_set.contains(&path_buf) {
                                    if let Ok(mode_num) = u32::from_str_radix(mode, 8) {
                                        fixer_map.insert(path_buf, (mode_num, oid.to_string()));
                                    }
                                }
                            }
                        }
                    }
                }

                // Avoid excessive memory usage on very large files by short-circuiting
                // the merge logic when no fixer output exists for the path.
                const LARGE_STASH_FILE_BYTES: usize = 1_000_000; // 1 MiB

                // Track whether any file restoration failed so we can preserve the stash
                let mut restoration_failed = false;

                for p in stash_paths.iter() {
                    let path = PathBuf::from(p);
                    let path_str = p.to_string_lossy();
                    // Lightweight size probe for the worktree snapshot stored in the stash
                    // Check if this is an untracked file (exists in stash^3 but not in stash^1 or stash^2)
                    // Use silent commands to avoid noisy "exists on disk, but not in ref" errors
                    let is_untracked = *env::HK_STASH_UNTRACKED
                        && git_cmd_silent([
                            "cat-file",
                            "-e",
                            &format!("{}^3:{}", &stash_ref, path_str),
                        ])
                        .run()
                        .is_ok()
                        && git_cmd_silent([
                            "cat-file",
                            "-e",
                            &format!("{}^2:{}", &stash_ref, path_str),
                        ])
                        .run()
                        .is_err();

                    // Handle untracked files specially - just restore from stash^3
                    if is_untracked {
                        debug!(
                            "manual-unstash: restoring untracked file from stash^3 path={}",
                            display_path(&path)
                        );
                        if let Ok(contents) = git_read_raw([
                            "cat-file",
                            "-p",
                            &format!("{}^3:{}", &stash_ref, path_str),
                        ]) {
                            if let Err(err) = xx::file::write(&path, &contents) {
                                warn!(
                                    "failed to write untracked file {}: {err:?}",
                                    display_path(&path)
                                );
                                restoration_failed = true;
                            }
                        }
                        // Skip normal merge path for untracked files
                        continue;
                    }

                    let work_size: Option<usize> =
                        git_cmd(["cat-file", "-s", &format!("{}:{}", &stash_ref, path_str)])
                            .read()
                            .ok()
                            .and_then(|s| s.trim().parse::<usize>().ok());
                    let has_fixer = fixer_map.contains_key(&path);
                    if work_size.unwrap_or(0) >= LARGE_STASH_FILE_BYTES && !has_fixer {
                        debug!(
                            "manual-unstash: large file without fixer; restoring worktree snapshot directly path={} size={}",
                            display_path(&path),
                            work_size.unwrap_or(0)
                        );
                        if let Ok(contents) =
                            git_cmd(["cat-file", "-p", &format!("{}:{}", &stash_ref, path_str)])
                                .read()
                        {
                            if let Err(err) = xx::file::write(&path, &contents) {
                                warn!(
                                    "failed to write large worktree snapshot for {}: {err:?}",
                                    display_path(&path)
                                );
                                restoration_failed = true;
                            }
                        }
                        // Skip normal merge path for large files
                        continue;
                    }
                    // Worktree content and Base (HEAD at stash time) from stash
                    // Prefer saved worktree snapshot captured before stashing; fallback to stash blob
                    let work_pre = if let Some(map) = &self.saved_worktree {
                        map.get(&path).cloned()
                    } else {
                        None
                    }
                    .or_else(|| {
                        // Use `git cat-file -p` to preserve exact blob bytes, including EOF newline state
                        git_read_raw(["cat-file", "-p", &format!("{}:{}", &stash_ref, path_str)])
                            .ok()
                    });
                    // Parent ^1 of the stash commit points to the HEAD commit at stash time
                    let base_pre =
                        git_read_raw(["cat-file", "-p", &format!("{}^1:{}", &stash_ref, path_str)])
                            .ok();
                    // Parent ^2 is the index at stash time. Use this to detect whether the path had
                    // any unstaged changes then (worktree vs index).
                    let index_pre =
                        git_read_raw(["cat-file", "-p", &format!("{}^2:{}", &stash_ref, path_str)])
                            .ok();
                    // Fixer content from saved index blob
                    let fixer = fixer_map
                        .get(&path)
                        .and_then(|(_, oid)| git_read_raw(["cat-file", "-p", oid]).ok());

                    // Trace summaries of inputs for diagnostics (trace-level only)
                    {
                        let summarize = |name: &str, s: Option<&str>| {
                            if let Some(v) = s {
                                let len = v.len();
                                let hash = xx::hash::hash_to_str(&v);
                                let head = v
                                    .lines()
                                    .find(|l| !l.trim().is_empty())
                                    .unwrap_or("")
                                    .trim();
                                trace!(
                                    "manual-unstash: {name} len={} hash={} head={:?}",
                                    len,
                                    &hash[..8],
                                    head
                                );
                            } else {
                                trace!("manual-unstash: {name} NONE");
                            }
                        };
                        summarize("base", base_pre.as_deref());
                        summarize("index", index_pre.as_deref());
                        summarize("work", work_pre.as_deref());
                        summarize("fixer", fixer.as_deref());
                    }

                    // If base is absent (file did not exist in HEAD at stash time), treat as empty
                    let base = base_pre.as_deref().unwrap_or("");
                    let has_base = base_pre.is_some();
                    let has_fixer = fixer.is_some();
                    let has_work = work_pre.is_some();
                    // Merge relative to the INDEX snapshot at stash time when available.
                    // This ensures that fixer changes applied to staged content are preserved,
                    // while unstaged changes (worktree-only diffs relative to index) are kept.
                    let base_for_merge = index_pre.as_deref().unwrap_or(base);
                    let mut merged = merge::three_way_merge_hunks(
                        base_for_merge,
                        fixer.as_deref(),
                        work_pre.as_deref(),
                    );

                    // Special-case: if the only worktree difference relative to the index snapshot
                    // is a pure tail insertion, prefer the fixer result and append the tail.
                    if let (Some(f), Some(w), Some(i)) =
                        (fixer.as_deref(), work_pre.as_deref(), index_pre.as_deref())
                    {
                        // Try strict prefix first
                        let mut tail_opt = w.strip_prefix(i);
                        // If that fails, allow a single trailing newline discrepancy
                        if tail_opt.is_none() && i.ends_with('\n') {
                            tail_opt = w.strip_prefix(&i[..i.len().saturating_sub(1)]);
                        }
                        if let Some(tail) = tail_opt {
                            // If w == i (no tail), tail is empty; otherwise append tail to fixer
                            let mut combined = f.to_string();
                            if !tail.is_empty() {
                                combined.push_str(tail);
                            }
                            merged = combined;
                        }
                    }

                    // Preserve newline-only difference between worktree and index from stash time
                    // Compare the worktree snapshot against the INDEX snapshot from stash time
                    let newline_only_change = match (work_pre.as_deref(), index_pre.as_deref()) {
                        (Some(w), Some(i)) => {
                            let case1 = w.len() + 1 == i.len()
                                && i.ends_with('\n')
                                && &i[..i.len() - 1] == w;
                            let case2 = i.len() + 1 == w.len()
                                && w.ends_with('\n')
                                && &w[..w.len() - 1] == i;
                            if case1 || case2 {
                                debug!(
                                    "manual-unstash: newline-only change detected path={} w_len={} i_len={} case1={} case2={}",
                                    display_path(&path),
                                    w.len(),
                                    i.len(),
                                    case1,
                                    case2
                                );
                            } else {
                                debug!(
                                    "manual-unstash: newline-only change NOT detected path={} w_len={} i_len={} ends_w={} ends_i={} equal_trim_w={} equal_trim_i={}",
                                    display_path(&path),
                                    w.len(),
                                    i.len(),
                                    w.ends_with('\n'),
                                    i.ends_with('\n'),
                                    if w.ends_with('\n') {
                                        &w[..w.len() - 1] == i
                                    } else {
                                        false
                                    },
                                    if i.ends_with('\n') {
                                        &i[..i.len() - 1] == w
                                    } else {
                                        false
                                    }
                                );
                            }
                            case1 || case2
                        }
                        _ => false,
                    };
                    // Preserve EOF newline-only differences without discarding fixer changes.
                    if newline_only_change {
                        if let (Some(w), Some(i)) = (work_pre.as_deref(), index_pre.as_deref()) {
                            let w_has_nl = w.ends_with('\n');
                            let i_has_nl = i.ends_with('\n');
                            if w_has_nl && !i_has_nl {
                                if !merged.ends_with('\n') {
                                    merged.push('\n');
                                }
                            } else if !w_has_nl && i_has_nl {
                                while merged.ends_with('\n') {
                                    merged.pop();
                                }
                            }
                        }
                    }

                    // If there were no unstaged changes at stash time for this path
                    // (worktree identical to index), prefer writing the fixer result to the worktree
                    // so that files formatted by fixers (e.g., Prettier) appear in the worktree post-commit.
                    if !newline_only_change {
                        if let (Some(wc), Some(ic), Some(fc)) =
                            (work_pre.as_ref(), index_pre.as_ref(), fixer.as_ref())
                        {
                            if wc == ic {
                                merged = fc.clone();
                            }
                        }
                    }

                    // Determine which side the merged result matches
                    let mut chosen = "mixed";
                    if let Some(w) = work_pre.as_deref() {
                        if merged == w {
                            chosen = "worktree";
                        }
                    }
                    if chosen == "mixed" {
                        if let Some(f) = fixer.as_deref() {
                            if merged == f {
                                chosen = "fixer";
                            }
                        }
                    }
                    if chosen == "mixed" && merged == base {
                        chosen = "base";
                    }

                    debug!(
                        "manual-unstash: merge decision path={} has_base={} has_fixer={} has_work={} chosen={}",
                        display_path(&path),
                        has_base,
                        has_fixer,
                        has_work,
                        chosen
                    );
                    trace!(
                        "manual-unstash: merged len={} hash={}",
                        merged.len(),
                        &xx::hash::hash_to_str(&merged)[..8]
                    );
                    if let Err(err) = xx::file::write(&path, &merged) {
                        warn!(
                            "failed to write merged worktree for {}: {err:?}",
                            display_path(&path)
                        );
                        restoration_failed = true;
                    }
                    // If fixer differs from base, ensure index has fixer blob unless newline-only change
                    if newline_only_change {
                        debug!(
                            "manual-unstash: newline-only change; leaving index untouched path={}",
                            display_path(&path)
                        );
                    } else if let Some((mode, oid)) = fixer_map.get(&path) {
                        let mode_str = format!("{:o}", mode);
                        if let Err(err) = git_cmd(["update-index", "--cacheinfo"]) // set index blob
                            .arg(mode_str)
                            .arg(oid)
                            .arg(&path)
                            .run()
                        {
                            warn!("failed to set index for {}: {err:?}", display_path(&path));
                            restoration_failed = true;
                        } else {
                            debug!(
                                "manual-unstash: set index cacheinfo path={} mode={mode:o} oid={oid}",
                                display_path(&path),
                            );
                        }
                    } else {
                        debug!(
                            "manual-unstash: no fixer entry in saved index; leaving index as-is path={}",
                            display_path(&path)
                        );
                    }
                }
                // Only drop the stash if all file restorations succeeded
                if restoration_failed {
                    error!(
                        "Failed to restore some files from stash. Stash has been preserved at '{stash_ref}'."
                    );
                    error!(
                        "You can manually recover your changes with: git stash show {stash_ref} && git stash apply {stash_ref}"
                    );
                    // Keep the stash around and return an error
                    return Err(eyre!(
                        "Stash restoration failed - stash preserved at {stash_ref}"
                    ));
                } else {
                    // All files restored successfully, safe to drop the stash
                    if let Err(err) = git_cmd(["stash", "drop", &stash_ref]).run() {
                        warn!("failed to drop stash: {err:?}");
                    }
                }
            }
        }
        job.set_status(ProgressStatus::Done);
        // Clear saved snapshots now that we've restored
        self.saved_worktree = None;
        self.stash_commit = None;
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
            git_cmd(["add", "--"]).args(pathspecs).run()?;
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

            let output = git_read([
                "diff",
                "-z",
                "--name-only",
                "--diff-filter=ACMRTUXB",
                format!("{merge_base}..{to_ref}").as_str(),
            ])?;
            Ok(output
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect())
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct GitStatus {
    pub unstaged_files: BTreeSet<PathBuf>,
    pub staged_files: BTreeSet<PathBuf>,
    pub untracked_files: BTreeSet<PathBuf>,
    pub modified_files: BTreeSet<PathBuf>,
    // Staged classifications
    pub staged_added_files: BTreeSet<PathBuf>,
    pub staged_modified_files: BTreeSet<PathBuf>,
    pub staged_deleted_files: BTreeSet<PathBuf>,
    pub staged_renamed_files: BTreeSet<PathBuf>,
    pub staged_copied_files: BTreeSet<PathBuf>,
    // Unstaged classifications
    pub unstaged_modified_files: BTreeSet<PathBuf>,
    pub unstaged_deleted_files: BTreeSet<PathBuf>,
    pub unstaged_renamed_files: BTreeSet<PathBuf>,
}
