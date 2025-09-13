use std::{
    cell::OnceCell,
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
            let mut cmd = git_cmd(["ls-files", "-z"]);
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
            self.build_diff()?
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
                // Do not remove unrelated untracked files when stashing via patch-file
                checkout_opts.remove_untracked(false);
                checkout_opts.force();
                checkout_opts.update_index(true);
                repo.checkout_index(None, Some(&mut checkout_opts))
                    .wrap_err("failed to reset worktree for modified files")?;
            } else if !status.modified_files.is_empty() {
                let args = vec!["restore", "--worktree", "--"]
                    .into_iter()
                    .chain(status.modified_files.iter().map(|p| p.to_str().unwrap()))
                    .collect::<Vec<_>>();
                git_run(&args)?;
            }
        } else {
            job.prop("message", "Stashed unstaged changes with git stash");
        }
        job.set_status(ProgressStatus::Done);
        Ok(())
    }

    fn build_diff(&self) -> Result<Option<StashType>> {
        debug!("building diff for stash");
        let patch = if let Some(repo) = &self.repo {
            // essentially: git diff-index --ignore-submodules --binary --exit-code --no-color --no-ext-diff (git write-tree)
            let mut opts = git2::DiffOptions::new();
            // Do not include untracked in patch-file mode; we leave untracked files alone
            opts.show_binary(true);
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
            // Shell git path: build a patch without untracked contents
            let out =
                xx::process::sh("git diff --no-color --no-ext-diff --binary --ignore-submodules")?;
            if out.trim().is_empty() {
                return Ok(None);
            }
            out
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
                    let mut cmd = git_cmd(["stash", "push", "--keep-index", "-m", "hk"]);
                    if *env::HK_STASH_UNTRACKED {
                        cmd = cmd.arg("--include-untracked");
                    }
                    cmd.run()?;
                    Ok(Some(StashType::Git))
                }
            }
        } else {
            let mut cmd = git_cmd(["stash", "push", "--keep-index", "-m", "hk"]);
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
                debug!(
                    "applying patch to {} files: {:?}",
                    affected_files.len(),
                    affected_files
                );

                let apply_res = git_cmd(["apply", "--3way"]).arg(&patch_file).run();

                // Check for conflicts after apply attempt
                let mut had_conflicts = false;
                for f in &affected_files {
                    if let Ok(s) = xx::file::read_to_string(f) {
                        if s.contains("<<<<<<<") && s.contains(">>>>>>>") {
                            had_conflicts = true;
                            debug!("found conflict markers in {}", display_path(f));
                            break;
                        }
                    }
                }

                match apply_res {
                    Ok(_) => {
                        debug!("patch applied successfully");
                    }
                    Err(err) => {
                        if had_conflicts {
                            debug!(
                                "git apply --3way returned error but left conflicts for {}: {err:?}; proceeding to resolve",
                                display_path(&patch_file)
                            );
                        } else {
                            warn!(
                                "git apply --3way failed for {}: {err:?}; attempting plain apply",
                                display_path(&patch_file)
                            );
                            if let Err(err2) = git_cmd(["apply"]).arg(&patch_file).run() {
                                return Err(eyre!(
                                    "failed to apply patch (3-way and plain) {}: {err:?}; {err2:?}",
                                    display_path(&patch_file)
                                ));
                            }
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
                    // After resolving, re-stage only files that were previously staged to clear unmerged state
                    if previously_staged.contains(&f) {
                        if let Err(err) = git_cmd(["add", "--"]).arg(&f).run() {
                            warn!(
                                "failed to stage {} after resolving conflicts: {err:?}",
                                display_path(&f)
                            );
                        }
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
        let to_unstage: Vec<_> = staged_after
            .iter()
            .filter(|p| !previously_staged.contains(*p))
            .cloned()
            .collect();
        if !to_unstage.is_empty() {
            trace!(
                "resetting unintended staged files after stash: {:?}",
                &to_unstage
            );
            let _ = self.reset_paths(&to_unstage);
        }
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

    pub fn reset_paths(&self, pathspecs: &[PathBuf]) -> Result<()> {
        let pathspecs = pathspecs.iter().collect_vec();
        trace!("resetting (unstaging) files: {:?}", &pathspecs);
        // Use shell git to ensure consistent behavior with HEAD
        git_cmd(["reset", "--"]).args(pathspecs).run()?;
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
