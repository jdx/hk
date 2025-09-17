use std::{
    collections::BTreeSet,
    ffi::{CString, OsString},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::Result;
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

// removed: parse_paths_from_patch

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
    saved_index: Option<Vec<(u32, String, PathBuf)>>,
    // Paths restaged by steps during the hook run that must be preserved in the index
    // even after applying/restoring the stash. If present, we skip restoring their
    // pre-hook index entries so fixer changes remain staged.
    restaged_paths: BTreeSet<PathBuf>,
}

enum StashType {
    LibGit,
    Git,
    // (patch_path, backup_worktree_path, file_path)
    PatchFiles(Vec<(PathBuf, PathBuf, PathBuf)>),
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
            saved_index: None,
            restaged_paths: BTreeSet::new(),
        })
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
                    let exists = path.exists();
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
                    let exists = path.exists();
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
                    // Only include in aggregated `unstaged_files` when there is a worktree-side change
                    let worktree_changed = st == git2::Status::WT_NEW
                        || st == git2::Status::WT_MODIFIED
                        || st == git2::Status::WT_TYPECHANGE
                        || st == git2::Status::WT_DELETED
                        || st == git2::Status::WT_RENAMED;
                    if worktree_changed && exists {
                        unstaged_files.insert(path.clone());
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
                let exists = path.exists();
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
        files_subset: Option<&[PathBuf]>,
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
        // If we were provided a subset of files (e.g., staged files for the hook),
        // do not attempt to pre-detect which of them have unstaged changes. Always
        // ask git to stash worktree changes limited to those paths. This ensures
        // partially staged files are handled correctly even when detection is flaky
        // under pre-commit environments.
        if let Some(paths) = files_subset {
            // Refresh index stat info to avoid stale mtime/size causing mis-detection
            let _ = git_run(["update-index", "-q", "--refresh"]);
            // Detect whether there are any worktree-only changes limited to the requested paths.
            // This guards against cases where `git stash --keep-index -- <paths>` prints
            // "No unstaged changes to stash" even though there are partially-staged hunks.
            let mut any_unstaged = false;
            // 1) diff (worktree vs index)
            {
                let mut args: Vec<OsString> = vec![
                    "diff".into(),
                    "--name-only".into(),
                    "-z".into(),
                    "--no-ext-diff".into(),
                    "--ignore-submodules".into(),
                ];
                args.push("--".into());
                args.extend(paths.iter().map(|p| OsString::from(p.as_os_str())));
                let out = git_read(args).unwrap_or_default();
                any_unstaged |= out.split('\0').any(|s| !s.is_empty());
            }
            // 2) ls-files -m (modified in worktree)
            if !any_unstaged {
                let mut args: Vec<OsString> = vec!["ls-files".into(), "-m".into(), "-z".into()];
                args.push("--".into());
                args.extend(paths.iter().map(|p| OsString::from(p.as_os_str())));
                let out = git_read(args).unwrap_or_default();
                any_unstaged |= out.split('\0').any(|s| !s.is_empty());
            }
            // 3) Intersect with previously computed status for extra safety
            if !any_unstaged {
                any_unstaged = paths
                    .iter()
                    .any(|p| status.unstaged_modified_files.contains(p));
            }
            job.prop("message", "Running git stash for selected paths");
            job.update();
            self.stash = self.push_stash(Some(paths), status, method)?;

            if self.stash.is_none() {
                // If we detected worktree-only changes but couldn't form either a real stash
                // or a patch-file fallback, abort to avoid committing unintended hunks.
                if any_unstaged {
                    job.prop("message", "Failed to isolate unstaged changes; aborting");
                    job.set_status(ProgressStatus::Failed);
                    return Err(eyre!(
                        "detected unstaged changes for selected paths but could not create stash/patch"
                    ));
                }
                job.prop("message", "No unstaged changes to stash");
                job.prop("files", &0);
                job.set_status(ProgressStatus::Done);
                return Ok(());
            }
            job.prop("message", "Stashed unstaged changes");
            job.set_status(ProgressStatus::Done);
            return Ok(());
        }

        // Fallback: compute the set of files in the entire repo that have worktree-only changes
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
        let files_count = files_to_stash.len();
        job.prop("files", &files_count);
        if files_to_stash.is_empty() {
            job.prop("message", "No unstaged changes to stash");
            job.set_status(ProgressStatus::Done);
            return Ok(());
        }
        job.prop("message", "Running git stash");
        job.update();
        let subset_vec: Vec<PathBuf> = files_to_stash.iter().cloned().collect();
        self.stash = self.push_stash(Some(&subset_vec[..]), status, method)?;
        if self.stash.is_none() {
            job.prop("message", "No unstaged files to stash");
            job.set_status(ProgressStatus::Done);
            return Ok(());
        }
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
        method: StashMethod,
    ) -> Result<Option<StashType>> {
        // When a subset of paths is provided, filter out untracked paths. Passing untracked
        // paths as pathspecs to `git stash push` can fail with "did not match any file(s) known to git".
        let tracked_subset: Option<Vec<PathBuf>> = paths.map(|ps| {
            ps.iter()
                .filter(|p| !status.untracked_files.contains(*p))
                .cloned()
                .collect()
        });
        // If after filtering there are no tracked paths left, do not attempt a partial stash.
        if let Some(ref ts) = tracked_subset {
            if ts.is_empty() {
                return Ok(None);
            }
        }
        if let Some(repo) = &mut self.repo {
            let sig = repo.signature()?;
            let mut flags = git2::StashFlags::default();
            if *env::HK_STASH_UNTRACKED {
                flags.set(git2::StashFlags::INCLUDE_UNTRACKED, true);
            }
            flags.set(git2::StashFlags::KEEP_INDEX, true);
            // If partial paths requested, prefer a targeted patch-file stash for reliability
            if let Some(paths) = tracked_subset.as_deref() {
                self.create_patch_stash(paths)
            } else {
                match repo.stash_save(&sig, "hk", Some(flags)) {
                    Ok(_) => Ok(Some(StashType::LibGit)),
                    Err(e) => {
                        debug!("libgit2 stash failed, falling back to shell git: {e}");
                        let before = git_cmd(["rev-parse", "-q", "--verify", "refs/stash"])
                            .read()
                            .ok();
                        let mut cmd = git_cmd(["stash", "push", "--keep-index", "-m", "hk"]);
                        if *env::HK_STASH_UNTRACKED {
                            cmd = cmd.arg("--include-untracked");
                        }
                        cmd.run()?;
                        let after = git_cmd(["rev-parse", "-q", "--verify", "refs/stash"])
                            .read()
                            .ok();
                        if before != after {
                            Ok(Some(StashType::Git))
                        } else if matches!(method, StashMethod::PatchFile | StashMethod::Git) {
                            if let Some(paths) = tracked_subset.as_deref() {
                                self.create_patch_stash(paths)
                            } else {
                                Ok(None)
                            }
                        } else {
                            Ok(None)
                        }
                    }
                }
            }
        } else {
            let before = git_cmd(["rev-parse", "-q", "--verify", "refs/stash"])
                .read()
                .ok();
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
            let after = git_cmd(["rev-parse", "-q", "--verify", "refs/stash"])
                .read()
                .ok();
            if before != after {
                Ok(Some(StashType::Git))
            } else if matches!(method, StashMethod::PatchFile | StashMethod::Git) {
                if let Some(paths) = tracked_subset.as_deref() {
                    self.create_patch_stash(paths)
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
    }

    fn create_patch_stash(&mut self, paths: &[PathBuf]) -> Result<Option<StashType>> {
        // Create per-file patches and back up current worktree content for safety.
        let mut entries: Vec<(PathBuf, PathBuf, PathBuf)> = Vec::new();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        for (idx, p) in paths.iter().enumerate() {
            if !p.exists() {
                continue;
            }
            let path_str = match p.to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let patch = git_cmd(["diff", "--binary", "--no-color", "--", &path_str])
                .read()
                .unwrap_or_default();
            if patch.trim().is_empty() {
                continue;
            }
            // Write per-file patch
            let mut patch_path = std::env::temp_dir();
            patch_path.push(format!("hk-stash-{}-{}.patch", ts, idx));
            xx::file::write(&patch_path, &patch)?;
            // Backup current worktree content to allow clean fallback restore
            let mut backup_path = std::env::temp_dir();
            backup_path.push(format!("hk-stash-backup-{}-{}.bak", ts, idx));
            let _ = std::fs::copy(p, &backup_path);
            entries.push((patch_path, backup_path, p.clone()));
            // Remove the unstaged changes from worktree by checking out the index version per-file
            let _ = git_cmd(["checkout", "--"]).arg(&path_str).run();
        }
        if entries.is_empty() {
            return Ok(None);
        }
        Ok(Some(StashType::PatchFiles(entries)))
    }

    // removed: push_stash_keep_index_no_untracked helper

    pub fn capture_index(&mut self, paths: &[PathBuf]) -> Result<()> {
        if paths.is_empty() {
            self.saved_index = Some(vec![]);
            return Ok(());
        }
        let mut args: Vec<OsString> = vec!["ls-files".into(), "-s".into(), "-z".into()];
        args.push("--".into());
        args.extend(paths.iter().map(|p| OsString::from(p.as_os_str())));
        let out = git_read(args)?;
        let mut entries: Vec<(u32, String, PathBuf)> = vec![];
        for rec in out.split('\0').filter(|s| !s.is_empty()) {
            // format: mode SP oid SP stage TAB path
            // example: 100644 0123456789abcdef... 0\tpath/to/file
            if let Some((left, path)) = rec.split_once('\t') {
                let mut parts = left.split_whitespace();
                let mode = parts.next().unwrap_or("100644");
                let oid = parts.next().unwrap_or("");
                if !oid.is_empty() {
                    let mode = u32::from_str_radix(mode, 8).unwrap_or(0o100644);
                    entries.push((mode, oid.to_string(), PathBuf::from(path)));
                }
            }
        }
        self.saved_index = Some(entries);
        Ok(())
    }

    pub fn restore_index(&mut self) -> Result<()> {
        let Some(entries) = self.saved_index.take() else {
            return Ok(());
        };
        if entries.is_empty() {
            return Ok(());
        }
        for (mode, oid, path) in entries {
            let mode_str = format!("{:o}", mode);
            git_cmd(["update-index", "--cacheinfo"])
                .arg(mode_str)
                .arg(&oid)
                .arg(path)
                .run()?;
        }
        // Clear after restoration; a subsequent hook run will rebuild this set
        self.restaged_paths.clear();
        Ok(())
    }

    pub fn pop_stash(&mut self) -> Result<()> {
        let Some(diff) = self.stash.take() else {
            return Ok(());
        };
        let job: Arc<ProgressJob>;
        // Capture currently staged files (using porcelain to align with shell git operations)
        // so we can preserve the user's intent. After applying the stash, we'll only re-stage
        // files that were already staged before the apply. This prevents unrelated files from
        // getting staged due to conflict handling side-effects.
        let previously_staged: BTreeSet<PathBuf> = {
            let out = git_read([
                "status",
                "--porcelain",
                "--no-renames",
                "--untracked-files=all",
                "-z",
            ])
            .unwrap_or_default();
            let mut staged = BTreeSet::new();
            for entry in out.split('\0').filter(|s| !s.is_empty()) {
                let mut chars = entry.chars();
                let x = chars.next().unwrap_or_default();
                let _y = chars.next().unwrap_or_default();
                let path = PathBuf::from(chars.skip(1).collect::<String>());
                let is_modified =
                    |c: char| c == 'M' || c == 'T' || c == 'A' || c == 'R' || c == 'C';
                if is_modified(x) {
                    staged.insert(path);
                }
            }
            staged
        };

        match diff {
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
                // Apply the stash first; if there are conflicts, prefer the stash (unstaged) side.
                let apply_res = git_cmd(["stash", "apply"]).run();

                // Check git status after apply attempt to understand the state
                let status = match git_cmd([
                    "status",
                    "--porcelain",
                    "-z",
                    "--no-renames",
                    "--untracked-files=all",
                ])
                .read()
                {
                    Ok(s) => s,
                    Err(err) => {
                        warn!("failed to read git status: {err:?}");
                        String::new()
                    }
                };
                let conflicted = conflicted_files_from_porcelain(&status);

                // Handle different scenarios based on apply result and conflicts
                if !conflicted.is_empty() {
                    // Case 1: There are conflicts - resolve them preferring stash content
                    debug!("resolving {} conflicted files", conflicted.len());
                    for f in conflicted.iter() {
                        if let Err(err) = resolve_conflict_markers_preferring_theirs(f) {
                            warn!(
                                "failed to resolve conflict markers in {}: {err:?}",
                                display_path(f)
                            );
                        }
                        // Only re-stage files that were previously staged to clear unmerged state
                        if previously_staged.contains(f) {
                            if let Err(err) = git_cmd(["add", "--"]).arg(f).run() {
                                warn!(
                                    "failed to stage {} after resolving conflicts: {err:?}",
                                    display_path(f)
                                );
                            }
                        }
                    }
                    // Drop the stash since we've applied it (even with conflicts)
                    if let Err(err) = git_cmd(["stash", "drop"]).run() {
                        warn!("failed to drop stash after conflict resolution: {err:?}");
                    }
                } else if apply_res.is_err() {
                    // Case 2: Apply failed but no conflicts detected - stash is likely intact
                    warn!("git stash apply failed: {:?}", apply_res.unwrap_err());
                    // Don't try git stash pop here, as the stash is likely still intact
                    // and the error might be due to a non-conflicting issue
                    debug!("stash apply failed without conflicts - leaving stash intact");
                } else {
                    // Case 3: Apply succeeded - drop the stash
                    if let Err(err) = git_cmd(["stash", "drop"]).run() {
                        warn!("failed to drop stash after successful apply: {err:?}");
                    }
                }
            }
            StashType::PatchFiles(entries) => {
                job = ProgressJobBuilder::new()
                    .prop("message", "stash – Applying patch")
                    .start();
                // Apply each file's patch with a 3-way attempt and clean fallback on failure
                for (patch_path, backup_path, file_path) in entries.iter() {
                    let apply_res =
                        git_cmd(["apply", "--3way", "--recount", "--whitespace=nowarn"])
                            .arg(patch_path.clone())
                            .run();
                    if let Err(err) = apply_res {
                        warn!(
                            "3-way patch apply failed for {}: {err:?}",
                            display_path(file_path)
                        );
                        // Fallback: restore exact backed-up worktree content
                        if let Err(copy_err) = std::fs::copy(backup_path, file_path) {
                            warn!(
                                "failed to restore backup for {}: {copy_err:?}",
                                display_path(file_path)
                            );
                        }
                    }
                    let _ = std::fs::remove_file(patch_path);
                    let _ = std::fs::remove_file(backup_path);
                }
            }
        }
        // After applying the stash (with or without conflicts), ensure we don't leave
        // any newly staged files that the user hadn't staged before. This can happen
        // when `git stash apply` reports an error but partially applies changes.
        let out_after = git_read([
            "status",
            "--porcelain",
            "--no-renames",
            "--untracked-files=all",
            "-z",
        ])
        .unwrap_or_default();
        let mut staged_after: BTreeSet<PathBuf> = BTreeSet::new();
        for entry in out_after.split('\0').filter(|s| !s.is_empty()) {
            let mut chars = entry.chars();
            let x = chars.next().unwrap_or_default();
            let _y = chars.next().unwrap_or_default();
            let path = PathBuf::from(chars.skip(1).collect::<String>());
            let is_modified = |c: char| c == 'M' || c == 'T' || c == 'A' || c == 'R' || c == 'C';
            if is_modified(x) {
                staged_after.insert(path);
            }
        }
        // Only unstage files that were not previously staged AND not explicitly restaged by steps
        let to_unstage: Vec<_> = staged_after
            .iter()
            .filter(|p| !previously_staged.contains(*p) && !self.restaged_paths.contains(*p))
            .cloned()
            .collect();
        if !to_unstage.is_empty() {
            trace!(
                "resetting unintended staged files after stash: {:?}",
                &to_unstage
            );
            let _ = self.reset_paths(&to_unstage);
        }
        // Do not restage here; preserve only what was staged before apply + step-intended stages
        // Debug: show staged files after cleanup
        let out_clean = git_read([
            "status",
            "--porcelain",
            "--no-renames",
            "--untracked-files=all",
            "-z",
        ])
        .unwrap_or_default();
        let mut staged_clean = BTreeSet::new();
        for entry in out_clean.split('\0').filter(|s| !s.is_empty()) {
            let mut chars = entry.chars();
            let x = chars.next().unwrap_or_default();
            let _y = chars.next().unwrap_or_default();
            let path = PathBuf::from(chars.skip(1).collect::<String>());
            let is_modified = |c: char| c == 'M' || c == 'T' || c == 'A' || c == 'R' || c == 'C';
            if is_modified(x) {
                staged_clean.insert(path);
            }
        }
        let staged: Vec<_> = staged_clean.into_iter().collect();
        trace!("staged after stash: {:?}", staged);
        // Ensure index entries for originally staged paths remain exactly as before
        if let Err(err) = self.restore_index() {
            warn!("failed to restore exact index entries: {err:?}");
        }
        job.set_status(ProgressStatus::Done);
        Ok(())
    }

    pub fn add(&mut self, pathspecs: &[PathBuf]) -> Result<()> {
        let pathspecs = pathspecs.iter().collect_vec();
        // Record for index-restore preservation
        for p in &pathspecs {
            self.restaged_paths.insert((*p).clone());
        }
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

    pub fn reset_paths(&self, pathspecs: &[PathBuf]) -> Result<()> {
        let pathspecs = pathspecs.iter().collect_vec();
        trace!("resetting (unstaging) files: {:?}", &pathspecs);
        // Use shell git to ensure consistent behavior with HEAD
        git_cmd(["reset", "--"]).args(pathspecs).run()?;
        Ok(())
    }

    /// Ensure any paths explicitly restaged by steps remain staged in the index.
    /// This is called at the end of a hook run to guarantee fixer changes are committed.
    pub fn finalize_restaged(&mut self) -> Result<()> {
        if self.restaged_paths.is_empty() {
            return Ok(());
        }
        let restaged: Vec<PathBuf> = self.restaged_paths.iter().cloned().collect();
        git_cmd(["add", "--"])
            .args(restaged.iter().map(|p| p.as_path()))
            .run()?;
        self.restaged_paths.clear();
        Ok(())
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
