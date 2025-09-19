use std::{
    collections::BTreeSet,
    ffi::{CString, OsString},
    path::PathBuf,
};

use crate::Result;
use crate::merge;
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

pub struct Git {
    repo: Option<Repository>,
    stash: Option<StashType>,
    saved_index: Option<Vec<(u32, String, PathBuf)>>,
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
            saved_index: None,
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

        // Fast path: if a file subset is provided, always attempt a partial-path stash.
        if let Some(paths) = files_subset {
            let _ = git_run(["update-index", "-q", "--refresh"]);
            job.prop("message", "Running git stash for selected paths");
            job.prop("files", &paths.len());
            job.update();
            self.stash = self.push_stash(Some(paths), status)?;
            if self.stash.is_some() {
                job.prop("message", "Stashed unstaged changes");
            } else {
                job.prop("message", "No unstaged changes to stash");
                job.prop("files", &0);
            }
            job.set_status(ProgressStatus::Done);
            return Ok(());
        }
        // Hardened detection of worktree-only changes (including partially staged files)
        let mut files_to_stash: BTreeSet<PathBuf> = BTreeSet::new();
        // 1) git diff --name-only (worktree vs index)
        {
            let mut args: Vec<OsString> = vec![
                "diff".into(),
                "--name-only".into(),
                "-z".into(),
                "--no-ext-diff".into(),
                "--ignore-submodules".into(),
            ];
            if let Some(paths) = files_subset {
                args.push("--".into());
                args.extend(paths.iter().map(|p| OsString::from(p.as_os_str())));
            }
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
            let mut args: Vec<OsString> = vec!["ls-files".into(), "-m".into(), "-z".into()];
            if let Some(paths) = files_subset {
                args.push("--".into());
                args.extend(paths.iter().map(|p| OsString::from(p.as_os_str())));
            }
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
            let mut args: Vec<OsString> = vec![
                "status".into(),
                "--porcelain".into(),
                "--no-renames".into(),
                "--untracked-files=all".into(),
                "-z".into(),
            ];
            if let Some(paths) = files_subset {
                args.push("--".into());
                args.extend(paths.iter().map(|p| OsString::from(p.as_os_str())));
            }
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
        // If a subset was provided, filter down to it explicitly (defense-in-depth)
        if let Some(paths) = files_subset {
            let allow: BTreeSet<PathBuf> = paths.iter().cloned().collect();
            files_to_stash.retain(|p| allow.contains(p));
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
                Ok(Some(StashType::Git))
            } else {
                match repo.stash_save(&sig, "hk", Some(flags)) {
                    Ok(_) => Ok(Some(StashType::LibGit)),
                    Err(e) => {
                        debug!("libgit2 stash failed, falling back to shell git: {e}");
                        let mut cmd = git_cmd(["stash", "push", "--keep-index", "-m", "hk"]);
                        if *env::HK_STASH_UNTRACKED {
                            cmd = cmd.arg("--include-untracked");
                        }
                        cmd.run()?;
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
            Ok(Some(StashType::Git))
        }
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
            // example: 100644 0123456789abcdef... 0	path/to/file
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

    pub fn pop_stash(&mut self) -> Result<()> {
        let Some(diff) = self.stash.take() else {
            return Ok(());
        };
        let job = ProgressJobBuilder::new()
            .prop("message", "stash – Restoring unstaged changes (manual)")
            .start();
        match diff {
            StashType::LibGit | StashType::Git => {
                // List paths in the top stash
                let show = git_cmd(["stash", "show", "--name-only", "-z", "stash@{0}"]) // top of stack
                    .read()
                    .unwrap_or_default();
                let stash_paths: Vec<PathBuf> = show
                    .split('\0')
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .collect();

                // Build a map of CURRENT index (post-step) entries to re-stage Fixer blobs
                let mut fixer_map: std::collections::HashMap<PathBuf, (u32, String)> =
                    std::collections::HashMap::new();
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
                                if !oid.is_empty() {
                                    if let Ok(mode_num) = u32::from_str_radix(mode, 8) {
                                        fixer_map.insert(
                                            PathBuf::from(path),
                                            (mode_num, oid.to_string()),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                for p in stash_paths.iter() {
                    let path = PathBuf::from(p);
                    let path_str = p.to_string_lossy();
                    // Worktree content and Base (HEAD at stash time) from stash
                    let work_pre = git_cmd(["show", &format!("stash@{{0}}:{}", path_str)])
                        .read()
                        .ok();
                    // Parent ^1 of the stash commit points to the HEAD commit at stash time
                    let base_pre = git_cmd(["show", &format!("stash@{{0}}^1:{}", path_str)])
                        .read()
                        .ok();
                    // Fixer content from saved index blob
                    let fixer = fixer_map
                        .get(&path)
                        .and_then(|(_, oid)| git_cmd(["cat-file", "-p", oid]).read().ok());

                    // If base is absent (file did not exist in HEAD at stash time), treat as empty
                    let base = base_pre.as_deref().unwrap_or("");
                    let has_base = base_pre.is_some();
                    let has_fixer = fixer.is_some();
                    let has_work = work_pre.is_some();
                    let merged =
                        merge::three_way_merge_hunks(base, fixer.as_deref(), work_pre.as_deref());

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
                    if let Err(err) = xx::file::write(&path, &merged) {
                        warn!(
                            "failed to write merged worktree for {}: {err:?}",
                            display_path(&path)
                        );
                    }
                    // If fixer differs from base, ensure index has fixer blob
                    if let Some((mode, oid)) = fixer_map.get(&path) {
                        let mode_str = format!("{:o}", mode);
                        if let Err(err) = git_cmd(["update-index", "--cacheinfo"]) // set index blob
                            .arg(mode_str)
                            .arg(oid)
                            .arg(&path)
                            .run()
                        {
                            warn!("failed to set index for {}: {err:?}", display_path(&path));
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
                // Drop the stash we manually consumed
                if let Err(err) = git_cmd(["stash", "drop"]).run() {
                    warn!("failed to drop stash: {err:?}");
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
