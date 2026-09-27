use std::{
    collections::BTreeSet,
    ffi::{CString, OsString},
    path::PathBuf,
    process::Command,
    sync::OnceLock,
    thread,
    time::Duration,
};

use crate::Result;
use crate::merge;
use crate::settings::Settings;
use crate::ui::style;
use clx::progress::{ProgressJob, ProgressJobBuilder, ProgressStatus};
use eyre::{WrapErr, eyre};
use git2::{Diff, ErrorCode, Repository, StatusOptions, StatusShow};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use xx::file::display_path;

use crate::env;

/// Returns true if the given string is git's all-zeros sha sentinel.
///
/// Git uses this to denote a missing ref (e.g., a deletion or a new branch
/// in pre-push stdin). The length depends on the repository's hash algorithm
/// — 40 chars for SHA-1, 64 for SHA-256 — so check the contents rather than
/// comparing against a fixed-width constant.
pub fn is_zero_sha(sha: &str) -> bool {
    !sha.is_empty() && sha.bytes().all(|b| b == b'0')
}

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

fn run_git_stash(cmd: &xx::process::XXExpression) -> Result<()> {
    const LOCK_RETRY_DELAYS: [Duration; 5] = [
        Duration::from_millis(25),
        Duration::from_millis(50),
        Duration::from_millis(100),
        Duration::from_millis(200),
        Duration::from_millis(400),
    ];

    let index_lock = git_cmd_silent(["rev-parse", "--git-path", "index.lock"])
        .read()
        .ok()
        .map(PathBuf::from);

    if let Some(index_lock) = index_lock {
        for delay in LOCK_RETRY_DELAYS {
            if !index_lock.exists() {
                break;
            }
            debug!(
                "waiting {}ms for transient git index lock {}",
                delay.as_millis(),
                index_lock.display()
            );
            thread::sleep(delay);
        }
    }

    cmd.run()?;
    Ok(())
}

fn git_read<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    Ok(git_cmd(args).read()?)
}

fn git_read_bytes<I, S>(args: I) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    // Don't use git_cmd here because it adds stderr handlers which breaks stdout capture
    let args = args.into_iter().map(|s| s.into()).collect::<Vec<_>>();
    let output = xx::process::cmd("git", args).stdout_capture().run()?;
    Ok(output.stdout)
}

/// Runs git and splits its NUL-separated output into paths, setting names
/// that are not valid UTF-8, which hk cannot handle as paths, aside as bytes.
fn git_read_paths<I, S>(args: I) -> Result<(Vec<PathBuf>, Vec<Vec<u8>>)>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut paths = Vec::new();
    let mut unnamed = Vec::new();
    for name in git_read_bytes(args)?.split(|&b| b == 0) {
        if name.is_empty() {
            continue;
        }
        match std::str::from_utf8(name) {
            Ok(path) => paths.push(PathBuf::from(path)),
            Err(_) => unnamed.push(name.to_vec()),
        }
    }
    Ok((paths, unnamed))
}

/// A path git printed, which need not be valid UTF-8.
fn path_from_raw(name: &[u8]) -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from(OsString::from_vec(name.to_vec()))
    }
    #[cfg(not(unix))]
    {
        // Git for Windows writes paths as UTF-8
        PathBuf::from(String::from_utf8_lossy(name).into_owned())
    }
}

/// The worktree changes a stash set aside.
#[derive(Debug, Default)]
struct StashedChanges {
    /// Tracked paths whose stashed worktree differs from the stashed index,
    /// with their modes in the index and in the worktree (0 where absent)
    modes: std::collections::BTreeMap<PathBuf, (u32, u32)>,
    /// Untracked files
    untracked: BTreeSet<PathBuf>,
    /// Paths of either kind that are not valid UTF-8, as git printed them
    unnamed: Vec<Vec<u8>>,
    /// The untracked ones among `unnamed`
    unnamed_untracked: BTreeSet<PathBuf>,
    /// The modes of the tracked ones among `unnamed`, as in `modes`
    unnamed_modes: std::collections::BTreeMap<PathBuf, (u32, u32)>,
}

impl StashedChanges {
    /// Why restoring the stash would destroy something a step did, for each
    /// path where it would.
    ///
    /// Stashing leaves the worktree as the stashed index (`^2`) with no
    /// untracked files, so a step touched a path exactly when the worktree
    /// differs from that index there. Restoring the stash over a path a step
    /// did not touch is safe, whatever the stash has there. A regular file
    /// that a step changed and that the stash also has as a regular file is
    /// merged with the stashed edits. Any other path a step touched, or a
    /// parent directory of one, conflicts with the stash.
    fn restore_conflicts(
        &self,
        stash_ref: &str,
        skip: &std::collections::HashSet<PathBuf>,
    ) -> Result<Vec<String>> {
        let index = IndexTree::read(&format!("{stash_ref}^2"))?;
        // Each path to restore, with its index and worktree modes when it is
        // tracked, and whether a changed regular file can be merged
        type ToRestore<'a> = (&'a PathBuf, Option<(u32, u32)>, bool);
        let mut paths: Vec<ToRestore> = Vec::new();
        // Restoring leaves the paths in `skip` alone
        paths.extend(
            self.modes
                .iter()
                .filter(|(path, _)| !skip.contains(*path))
                .map(|(path, modes)| (path, Some(*modes), true)),
        );
        // hk passed no step the paths that are not valid UTF-8
        paths.extend(
            self.unnamed_modes
                .iter()
                .map(|(path, modes)| (path, Some(*modes), false)),
        );
        paths.extend(
            self.untracked
                .iter()
                .chain(self.unnamed_untracked.iter())
                .map(|path| (path, None, false)),
        );
        let parents = |path: &PathBuf| {
            let mut parents = path
                .ancestors()
                .skip(1)
                .filter(|p| !p.as_os_str().is_empty())
                .map(std::path::Path::to_path_buf)
                .collect_vec();
            parents.reverse();
            parents
        };
        let untouched = index.worktree_matcher(
            paths
                .iter()
                .flat_map(|(path, _, _)| parents(path).into_iter().chain([(*path).clone()])),
        )?;

        let mut conflicts = Vec::new();
        'paths: for (path, modes, mergeable) in paths {
            for parent in parents(path) {
                let Ok(metadata) = std::fs::symlink_metadata(&parent) else {
                    // Neither it nor anything under it exists
                    break;
                };
                // A directory a step created only receives restored files
                let created_dir = index.get(&parent).is_none() && metadata.is_dir();
                if !created_dir && !untouched(&parent) {
                    conflicts.push(format!(
                        "a step changed {}, where the stash has a directory",
                        display_path(&parent)
                    ));
                    continue 'paths;
                }
            }
            if untouched(path) {
                continue;
            }
            let is_regular = |mode: u32| matches!(mode, 0o100644 | 0o100755);
            let merged = mergeable
                && modes.is_some_and(|(old_mode, new_mode)| {
                    is_regular(old_mode) && is_regular(new_mode)
                })
                && std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file());
            if !merged {
                conflicts.push(format!(
                    "a step changed {}, which the stash also changed",
                    display_path(path)
                ));
            }
        }
        Ok(conflicts)
    }
}

/// The entries of an index tree, with their modes and object names.
struct IndexTree {
    entries: std::collections::HashMap<PathBuf, (u32, String)>,
}

impl IndexTree {
    fn read(tree: &str) -> Result<Self> {
        let unexpected = || eyre!("unexpected git ls-tree output for {tree}");
        let mut entries = std::collections::HashMap::new();
        for record in git_read_bytes(["ls-tree", "-r", "-t", "-z", "--full-tree", tree])?
            .split(|&b| b == 0)
            .filter(|r| !r.is_empty())
        {
            // `<mode> SP <type> SP <object> TAB <path>`
            let tab = record
                .iter()
                .position(|&b| b == b'\t')
                .ok_or_else(unexpected)?;
            let header = std::str::from_utf8(&record[..tab]).map_err(|_| unexpected())?;
            let mut fields = header.split(' ');
            let mode = fields
                .next()
                .and_then(|m| u32::from_str_radix(m, 8).ok())
                .ok_or_else(unexpected)?;
            let object = fields.nth(1).ok_or_else(unexpected)?.to_string();
            entries.insert(path_from_raw(&record[tab + 1..]), (mode, object));
        }
        Ok(Self { entries })
    }

    fn get(&self, path: &std::path::Path) -> Option<&(u32, String)> {
        self.entries.get(path)
    }

    /// A check of whether the worktree still matches this index at a path:
    /// the same kind of entry, mode and contents, or no entry where the index
    /// has none. It hashes the regular files among `paths` up front.
    fn worktree_matcher(
        &self,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> Result<impl Fn(&std::path::Path) -> bool + '_> {
        let regular: BTreeSet<PathBuf> = paths
            .into_iter()
            .filter(|path| {
                self.get(path)
                    .is_some_and(|(mode, _)| matches!(mode, 0o100644 | 0o100755))
                    && std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file())
            })
            .collect();
        let hashes = hash_worktree_files(&regular)?;
        Ok(move |path: &std::path::Path| {
            let metadata = std::fs::symlink_metadata(path);
            match (self.get(path), metadata) {
                (None, Err(_)) => true,
                (None, Ok(_)) | (Some(_), Err(_)) => false,
                (Some((mode, object)), Ok(metadata)) => match *mode {
                    0o040000 => metadata.is_dir(),
                    0o120000 => {
                        metadata.file_type().is_symlink() && symlink_target_matches(path, object)
                    }
                    0o100644 | 0o100755 => {
                        metadata.is_file()
                            && executable_matches(&metadata, *mode)
                            && hashes.get(path) == Some(object)
                    }
                    // Submodules are not stashed
                    _ => true,
                },
            }
        })
    }
}

/// Git's object names for the worktree files at `paths`, with the same
/// filters as `git add`.
fn hash_worktree_files(
    paths: &BTreeSet<PathBuf>,
) -> Result<std::collections::HashMap<PathBuf, String>> {
    let mut hashes = std::collections::HashMap::new();
    // `--stdin-paths` takes one path per line
    let (batch, single): (Vec<&PathBuf>, Vec<&PathBuf>) = paths
        .iter()
        .partition(|p| !p.as_os_str().as_encoded_bytes().contains(&b'\n'));
    if !batch.is_empty() {
        let mut input = Vec::new();
        for path in &batch {
            input.extend_from_slice(path.as_os_str().as_encoded_bytes());
            input.push(b'\n');
        }
        let output = xx::process::cmd("git", ["hash-object", "--stdin-paths"])
            .stdin_bytes(input)
            .stdout_capture()
            .run()?
            .stdout;
        let objects = String::from_utf8(output)?;
        let objects = objects.lines().collect_vec();
        if objects.len() != batch.len() {
            return Err(eyre!("unexpected git hash-object output"));
        }
        for (path, object) in batch.into_iter().zip(objects) {
            hashes.insert(path.clone(), object.to_string());
        }
    }
    for path in single {
        let output = git_read_bytes([
            OsString::from("hash-object"),
            "--".into(),
            path.as_os_str().to_owned(),
        ])?;
        hashes.insert(path.clone(), String::from_utf8(output)?.trim().to_string());
    }
    Ok(hashes)
}

/// Whether the symlink at `path` points where the blob `object` says.
fn symlink_target_matches(path: &std::path::Path, object: &str) -> bool {
    let Ok(target) = std::fs::read_link(path) else {
        return false;
    };
    git_read_bytes(["cat-file", "blob", object])
        .is_ok_and(|blob| blob == target.as_os_str().as_encoded_bytes())
}

/// Whether a file's executable bit matches a git file mode, where the
/// filesystem has one.
fn executable_matches(metadata: &std::fs::Metadata, mode: u32) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        (metadata.permissions().mode() & 0o111 != 0) == (mode == 0o100755)
    }
    #[cfg(not(unix))]
    {
        let _ = (metadata, mode);
        true
    }
}

/// Lists what `stash_ref` set aside: how its worktree tree differs from its
/// index tree (`^2`), and its untracked files (`^3`).
fn stashed_changes(stash_ref: &str) -> Result<StashedChanges> {
    let mut changes = StashedChanges::default();
    let unexpected = || eyre!("unexpected git diff output for {stash_ref}");
    let index = format!("{stash_ref}^2");
    let raw = git_read_bytes([
        "diff",
        "--raw",
        "-z",
        "--no-renames",
        "--no-ext-diff",
        "--ignore-submodules",
        &index,
        stash_ref,
    ])?;
    let mut fields = raw.split(|&b| b == 0);
    while let Some(header) = fields.next() {
        if header.is_empty() {
            continue;
        }
        let path = fields.next().ok_or_else(unexpected)?;
        // `:<index mode> <worktree mode> <index object> <worktree object> <status>`
        let header = std::str::from_utf8(header).map_err(|_| unexpected())?;
        let mut modes = header
            .trim_start_matches(':')
            .split(' ')
            .take(2)
            .map(|m| u32::from_str_radix(m, 8).map_err(|_| unexpected()));
        let old_mode = modes.next().ok_or_else(unexpected)??;
        let new_mode = modes.next().ok_or_else(unexpected)??;
        match std::str::from_utf8(path) {
            Ok(path) => {
                changes
                    .modes
                    .insert(PathBuf::from(path), (old_mode, new_mode));
            }
            Err(_) => {
                changes
                    .unnamed_modes
                    .insert(path_from_raw(path), (old_mode, new_mode));
                changes.unnamed.push(path.to_vec());
            }
        }
    }
    let untracked = format!("{stash_ref}^3");
    if git_cmd_silent(["rev-parse", "-q", "--verify", &untracked])
        .read()
        .is_ok()
    {
        let (paths, unnamed) = git_read_paths(["ls-tree", "-r", "-z", "--name-only", &untracked])?;
        changes.untracked.extend(paths);
        for name in unnamed {
            changes.unnamed_untracked.insert(path_from_raw(&name));
            changes.unnamed.push(name);
        }
    }
    Ok(changes)
}

/// What a stash holds at a path it set aside.
#[derive(Debug, Clone, Copy)]
enum StashedEntry {
    /// An untracked file, in the stash's third parent
    Untracked,
    /// A tracked file or symlink
    Tracked,
    /// Nothing: the file was deleted from the worktree
    Deleted,
}

impl StashedChanges {
    /// What the stash holds at `path`, one of the paths it set aside.
    fn entry(&self, path: &std::path::Path) -> StashedEntry {
        if self.untracked.contains(path) || self.unnamed_untracked.contains(path) {
            return StashedEntry::Untracked;
        }
        let modes = self.modes.get(path).or(self.unnamed_modes.get(path));
        if modes.is_some_and(|(_, new_mode)| *new_mode == 0) {
            StashedEntry::Deleted
        } else {
            StashedEntry::Tracked
        }
    }
}

/// Restores a path to its state in `stash_ref`: its untracked or tracked
/// contents and mode, or its deletion.
fn restore_stashed_path(
    stash_ref: &str,
    name: &std::ffi::OsStr,
    entry: StashedEntry,
) -> Result<()> {
    let source = match entry {
        StashedEntry::Untracked => format!("{stash_ref}^3"),
        StashedEntry::Tracked => stash_ref.to_string(),
        StashedEntry::Deleted => {
            // Stashing brought the index version back
            let path = std::path::Path::new(name);
            if std::fs::symlink_metadata(path).is_ok() {
                std::fs::remove_file(path)?;
            }
            return Ok(());
        }
    };
    let mut pathspec = OsString::from(":(literal)");
    pathspec.push(name);
    // Writes the worktree only, with the stashed file mode
    git_cmd([
        OsString::from("restore"),
        format!("--source={source}").into(),
        "--worktree".into(),
        "--".into(),
        pathspec,
    ])
    .run()?;
    Ok(())
}

/// The restored paths among `restored` that still differ from `stash_ref`.
fn unrestored_paths(
    stash_ref: &str,
    changes: &StashedChanges,
    restored: &BTreeSet<PathBuf>,
) -> Result<Vec<PathBuf>> {
    // Tracked paths whose worktree differs from the stashed worktree
    let differ: BTreeSet<PathBuf> = git_read_bytes([
        "diff",
        "--name-only",
        "-z",
        "--no-renames",
        "--no-ext-diff",
        "--ignore-submodules",
        stash_ref,
    ])?
    .split(|&b| b == 0)
    .filter(|name| !name.is_empty())
    .map(path_from_raw)
    .collect();
    Ok(restored
        .iter()
        .filter(|path| {
            if changes.untracked.contains(*path) || changes.unnamed_untracked.contains(*path) {
                std::fs::symlink_metadata(path).is_err()
            } else {
                differ.contains(*path)
            }
        })
        .cloned()
        .collect())
}

fn is_symlink_mode(mode: u32) -> bool {
    mode == 0o120000
}

/// Sets or clears the executable bits of the regular file at `path` for a
/// git file mode, as git checks it out.
fn set_file_mode(path: &std::path::Path, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(metadata) = std::fs::symlink_metadata(path) else {
            return Ok(());
        };
        if !metadata.is_file() {
            return Ok(());
        }
        let perms = metadata.permissions().mode();
        let new_perms = match mode {
            0o100755 => perms | ((perms & 0o444) >> 2),
            0o100644 => perms & !0o111,
            _ => return Ok(()),
        };
        if new_perms != perms {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(new_perms))?;
        }
    }
    #[cfg(not(unix))]
    let _ = (path, mode);
    Ok(())
}

fn git_read_raw<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let bytes = git_read_bytes(args)?;
    String::from_utf8(bytes).map_err(|err| eyre!("git output is not valid UTF-8: {err}"))
}

pub struct Git {
    repo: Option<Repository>,
    stash: Option<StashType>,
    // Commit id of the stash entry we created (top-of-stack at creation time)
    stash_commit: Option<String>,
    stashed_paths: Option<BTreeSet<PathBuf>>,
    saved_index: Option<Vec<(u32, String, PathBuf)>>,
    saved_worktree: Option<std::collections::HashMap<PathBuf, String>>,
    // Path of the most recent stash patch backup, surfaced if restore fails
    last_patch_path: Option<PathBuf>,
    // Path of the index file git writes, resolved on first use
    index_path: OnceLock<PathBuf>,
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
        // Respect GIT_DIR / GIT_WORK_TREE so hk works with bare-repo dotfile
        // managers like YADM where there is no `.git` in the work tree.
        let has_git_env =
            std::env::var_os("GIT_DIR").is_some() || std::env::var_os("GIT_WORK_TREE").is_some();

        let root = if has_git_env {
            // Absolutize relative GIT_DIR / GIT_WORK_TREE before we change
            // directory, otherwise libgit2 and downstream git commands will
            // resolve them against the new cwd and look in the wrong place.
            let cwd = std::env::current_dir()?;
            for var in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
                if let Some(val) = std::env::var_os(var) {
                    let p = std::path::Path::new(&val);
                    if p.is_relative() {
                        // SAFETY: set_var is only unsafe because other threads
                        // may read the environment concurrently; we run this
                        // before any worker threads are spawned.
                        unsafe { std::env::set_var(var, cwd.join(p)) };
                    }
                }
            }
            crate::git_util::find_work_tree_root()
        } else {
            let cwd = std::env::current_dir()?;
            xx::file::find_up(&cwd, &[".git"])
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .ok_or(eyre!("failed to find git repository"))?
        };
        // Always cd into the work tree root so libgit2 and shell-git return
        // relative paths that resolve against the correct directory.
        std::env::set_current_dir(&root)?;
        let repo = if *env::HK_LIBGIT2 {
            debug!("libgit2: true");
            let repo = if has_git_env {
                Repository::open_from_env().wrap_err("failed to open repository")?
            } else {
                Repository::open(".").wrap_err("failed to open repository")?
            };
            // libgit2 status/diff APIs refuse to operate on a bare repository.
            // For bare-repo dotfile managers (YADM, etc.) the work tree is
            // provided via GIT_WORK_TREE but libgit2 still flags the repo as
            // bare — fall back to the shell-git path so those operations work.
            if repo.is_bare() {
                debug!("libgit2: bare repo detected, falling back to shell git");
                None
            } else {
                if let Some(index_file) = &*env::GIT_INDEX_FILE {
                    // sets index to .git/index.lock which is used in the case of `git commit -a`
                    let mut index =
                        git2::Index::open(index_file).wrap_err("failed to get index")?;
                    repo.set_index(&mut index)?;
                }
                Some(repo)
            }
        } else {
            debug!("libgit2: false");
            None
        };
        Ok(Self {
            repo,
            stash: None,
            stash_commit: None,
            stashed_paths: None,
            saved_index: None,
            saved_worktree: None,
            last_patch_path: None,
            index_path: OnceLock::new(),
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
                if let Ok(file_name) = entry.file_name().into_string()
                    && file_name.starts_with(&prefix)
                    && file_name.ends_with(".patch")
                    && let Ok(metadata) = entry.metadata()
                    && let Ok(modified) = metadata.modified()
                {
                    patch_files.push((entry.path(), modified));
                }
            }
        }

        // Sort by modification time, newest first
        patch_files.sort_by_key(|f| std::cmp::Reverse(f.1));

        // Remove old patches beyond keep_count
        for (path, _) in patch_files.iter().skip(keep_count) {
            debug!("Rotating old patch file: {}", path.display());
            let _ = std::fs::remove_file(path);
        }

        Ok(())
    }

    /// Save a patch backup of the stash
    fn save_stash_patch(&mut self, stash_ref: &str) {
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

        // Generate timestamp and haiku for unique, memorable filename
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        let haiku = xx::rand::haiku(&xx::rand::HaikuOptions {
            words: 1,
            ..Default::default()
        });

        let patch_filename = format!("{}-{}-{}.patch", repo_name, timestamp, haiku);
        let patch_path = patches_dir.join(&patch_filename);

        // Generate patch using git stash show
        let mut cmd = git_cmd_silent(["stash", "show", "-p"]);
        if *env::HK_STASH_UNTRACKED {
            cmd = cmd.arg("--include-untracked");
        }
        cmd = cmd.arg(stash_ref);

        // Read patch content from git
        let mut patch_content = match cmd.read() {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to generate stash patch: {}", e);
                return;
            }
        };
        // read() strips the trailing newline, but git apply rejects a patch
        // without one as corrupt, so restore it
        if !patch_content.is_empty() && !patch_content.ends_with('\n') {
            patch_content.push('\n');
        }

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
        self.last_patch_path = Some(patch_path);

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
        if let Ok(symref) = head_sym
            && let Some(target) = symref.lines().next()
        {
            // Expect something like: refs/remotes/main
            if let Some(short) = target.strip_prefix("refs/remotes/") {
                return Ok(short.to_string());
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
        if let Ok(cfg) = crate::config::Config::get()
            && let Some(val) = cfg.default_branch
            && !val.trim().is_empty()
        {
            return val;
        }
        self.default_branch().unwrap_or_else(|_| "main".to_string())
    }
    // removed: patch_file path helper

    pub fn matching_remote_branch(&self, remote: &str) -> Result<Option<String>> {
        if let Some(branch) = self.current_branch()? {
            if let Some(repo) = &self.repo {
                if let Ok(_ref) = repo.find_reference(&format!("refs/remotes/{remote}/{branch}")) {
                    return Ok(_ref.name().ok().map(|s| s.to_string()));
                }
            } else {
                let output = git_read([
                    "ls-remote",
                    "--heads",
                    "--end-of-options",
                    remote,
                    branch.as_str(),
                ])?;
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
            let branch_name = head.shorthand().ok().map(|s| s.to_string());
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

    /// Status of the whole repository, after refreshing the index's stat
    /// information so that files whose mtime changed but whose contents did
    /// not are not reported as modified.
    ///
    /// This runs `git status` even with libgit2. libgit2 compares HEAD with
    /// the index and then the index with the worktree in two single-threaded
    /// passes over every entry, while git stats the worktree with several
    /// threads and skips unchanged directories through the cache tree. With
    /// optional locks allowed, `git status` also writes the refreshed index
    /// back when it can take the lock, as `git update-index --refresh` would.
    #[tracing::instrument(level = "info", name = "git.status", skip_all)]
    pub fn status(&self) -> Result<GitStatus> {
        let include_untracked = *env::HK_STASH_UNTRACKED;
        let mut args = vec![
            "status",
            "--porcelain=v2",
            "-z",
            untracked_files_arg(include_untracked),
        ];
        if self.repo.is_some() {
            // libgit2 detects staged renames whatever `status.renames` says
            args.push("--renames");
        }
        let output = git_read_bytes(args).wrap_err("failed to get git status")?;
        let mut skipped = SkippedPaths::default();
        let entries = parse_porcelain_status(&output, &mut skipped)?;
        let mut status = if self.repo.is_some() {
            GitStatus::from_entries_libgit2(entries, &mut skipped)
        } else {
            GitStatus::from_entries(entries)
        };
        status.set_skipped(&skipped);
        warn_skipped_paths(&skipped);
        Ok(status)
    }

    /// Status of the paths matching `pathspec`, read without touching any
    /// file outside them.
    ///
    /// Like [`Git::status_of_paths`], it skips the index refresh in
    /// [`Git::status`], which may hash any tracked file.
    #[tracing::instrument(level = "info", name = "git.status_of_pathspec", skip_all, fields(pathspec_count = pathspec.len()))]
    pub fn status_of_pathspec(&self, pathspec: &[OsString]) -> Result<GitStatus> {
        self.read_status(Some(pathspec), false)
    }

    /// Worktree paths that any index write may read, besides the paths being
    /// written.
    ///
    /// When git writes the index, it re-hashes every entry whose recorded mtime
    /// is not older than the index file ("racily clean") and whose stat data
    /// still matches the worktree, so it can tell whether the file changed
    /// within the same timestamp tick. git maps the file into memory to hash
    /// it, so if another process truncates the file meanwhile, git dies with
    /// SIGBUS. Entries recorded with size 0 were already marked as changed and
    /// are never hashed.
    ///
    /// Seconds are compared, which covers git builds with and without
    /// nanosecond timestamps.
    pub fn racily_clean_paths(&self) -> Result<Vec<PathBuf>> {
        let index_path = match self.index_path.get() {
            Some(path) => path,
            None => {
                let path = PathBuf::from(git_read(["rev-parse", "--git-path", "index"])?);
                self.index_path.get_or_init(|| path)
            }
        };
        let index_mtime = match std::fs::metadata(index_path) {
            Ok(metadata) => metadata.modified()?,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(err) => return Err(err.into()),
        };
        let index_secs = index_mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let index = git2::Index::open(index_path)
            .wrap_err_with(|| format!("failed to read index {}", index_path.display()))?;
        Ok(index
            .iter()
            .filter(|entry| entry.file_size != 0 && i64::from(entry.mtime.seconds()) >= index_secs)
            .map(|entry| {
                #[cfg(unix)]
                let path = PathBuf::from(OsString::from_vec(entry.path));
                #[cfg(not(unix))]
                let path = PathBuf::from(String::from_utf8_lossy(&entry.path).into_owned());
                path
            })
            .collect())
    }

    /// Status of exactly `paths`, read without touching any other file in the
    /// worktree, so it is safe while other steps are writing other files.
    ///
    /// It skips the index refresh in [`Git::status`], which stats and may hash
    /// every tracked file. Both libgit2 and `git status` compare contents
    /// themselves when an entry's stat information is stale.
    #[tracing::instrument(level = "info", name = "git.status_of_paths", skip_all, fields(path_count = paths.len()))]
    pub fn status_of_paths(&self, paths: &[PathBuf]) -> Result<GitStatus> {
        if paths.is_empty() {
            return Ok(GitStatus::default());
        }
        if self.repo.is_some() {
            let pathspec = paths.iter().map(|p| p.as_os_str().to_owned()).collect_vec();
            return self.read_status(Some(&pathspec), true);
        }
        // `git status` takes pathspecs only as arguments, so query in chunks
        // that stay well under the command-line limit (32 KiB on Windows).
        const MAX_ARG_BYTES: usize = 16 * 1024;
        let mut status = GitStatus::default();
        let mut chunk: Vec<OsString> = Vec::new();
        let mut chunk_bytes = 0;
        for p in paths {
            let mut spec = OsString::from(":(literal)");
            spec.push(p);
            if !chunk.is_empty() && chunk_bytes + spec.len() + 1 > MAX_ARG_BYTES {
                status.extend(self.read_status(Some(&chunk), true)?);
                chunk.clear();
                chunk_bytes = 0;
            }
            chunk_bytes += spec.len() + 1;
            chunk.push(spec);
        }
        status.extend(self.read_status(Some(&chunk), true)?);
        Ok(status)
    }

    /// `literal` matches libgit2 pathspecs as exact paths; git CLI callers mark
    /// literal pathspecs themselves with `:(literal)`.
    fn read_status(&self, pathspec: Option<&[OsString]>, literal: bool) -> Result<GitStatus> {
        // When stashing untracked files is disabled, skip the untracked-file scan.
        // This avoids catastrophic scans when GIT_WORK_TREE points at a large tree
        // (e.g. YADM dotfile repos where the worktree is $HOME). See #860.
        let include_untracked = *env::HK_STASH_UNTRACKED;
        if let Some(repo) = &self.repo {
            let mut status_options = StatusOptions::new();
            status_options.include_untracked(include_untracked);
            status_options.recurse_untracked_dirs(include_untracked);
            status_options.renames_head_to_index(true);
            status_options.disable_pathspec_match(literal);

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
                let st = s.status();
                // For renamed entries, s.path() returns the old path which no longer
                // exists in the worktree; use the new path from the head-to-index delta
                let path = if st.is_index_renamed() {
                    // INDEX_RENAMED implies a head-to-index rename delta, whose
                    // new_file path is always present
                    s.head_to_index()
                        .and_then(|d| d.new_file().path())
                        .map(PathBuf::from)
                } else {
                    s.path().map(PathBuf::from).ok()
                };
                if let Some(path) = path {
                    // Check if path exists (including broken symlinks)
                    // path.exists() returns false for broken symlinks, but symlink_metadata succeeds
                    let exists = path.exists() || std::fs::symlink_metadata(&path).is_ok();
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
                if let Ok(path) = s.path().map(PathBuf::from) {
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
                        // Note: s.path() would be the old path here, but WT_RENAMED
                        // is unreachable while renames_index_to_workdir is not enabled
                        unstaged_renamed_files.insert(path.clone());
                    }
                    if exists && st != git2::Status::WT_NEW {
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
                // libgit2 skips paths that are not valid UTF-8 without saying
                // so, but these are queries for paths hk already knows
                skipped_unstaged: false,
                skipped_untracked: false,
            })
        } else {
            let mut args = vec![
                OsString::from("status"),
                "--porcelain=v2".into(),
                untracked_files_arg(include_untracked).into(),
                "-z".into(),
            ];
            if let Some(pathspec) = pathspec {
                args.push("--".into());
                args.extend(pathspec.iter().cloned());
            }
            // With optional locks, `git status` writes the refreshed index
            // back, and writing the index re-hashes every racily clean entry
            // in the repository, not just the ones in `pathspec`.
            // The output is read as bytes: paths need not be valid UTF-8.
            let output = xx::process::cmd("git", args)
                .env("GIT_OPTIONAL_LOCKS", "0")
                .stdout_capture()
                .run()?
                .stdout;
            let mut skipped = SkippedPaths::default();
            let mut status =
                GitStatus::from_entries(parse_porcelain_status(&output, &mut skipped)?);
            status.set_skipped(&skipped);
            warn_skipped_paths(&skipped);
            Ok(status)
        }
    }

    #[tracing::instrument(level = "info", name = "git.stash.push", skip_all)]
    /// Paths whose unstaged changes the last [`Git::stash_unstaged`] set aside,
    /// or `None` if it stashed nothing.
    pub fn stashed_paths(&self) -> Option<&BTreeSet<PathBuf>> {
        self.stash.as_ref().and(self.stashed_paths.as_ref())
    }

    pub fn stash_unstaged(
        &mut self,
        job: &ProgressJob,
        method: StashMethod,
        status: &GitStatus,
    ) -> Result<()> {
        // Skip stashing if auto-stash is disabled or there's no initial commit yet
        if method == StashMethod::None {
            return Ok(());
        }
        let has_head = match &self.repo {
            Some(repo) => repo.head().is_ok(),
            None => git_cmd_silent(["rev-parse", "--verify", "-q", "HEAD"])
                .read()
                .is_ok(),
        };
        if !has_head {
            return Ok(());
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
            // Paths that are not valid UTF-8 are left out of the status, which
            // records that they need stashing
            let (paths, _) = git_read_paths(args).unwrap_or_default();
            files_to_stash.extend(paths.into_iter().filter(|p| p.exists()));
        }
        // 2) git ls-files -m (modified in worktree)
        {
            let args: Vec<OsString> = vec!["ls-files".into(), "-m".into(), "-z".into()];
            let (paths, _) = git_read_paths(args).unwrap_or_default();
            files_to_stash.extend(paths.into_iter().filter(|p| p.exists()));
        }
        // 3) Parse porcelain to catch nuanced mixed states.
        // We only look at worktree-side markers (M/T/R), so untracked entries
        // are irrelevant here. Skip the untracked scan entirely when
        // HK_STASH_UNTRACKED=false to avoid scanning a huge worktree (see #860).
        {
            let untracked_arg = if *env::HK_STASH_UNTRACKED {
                "--untracked-files=all"
            } else {
                "--untracked-files=no"
            };
            let args: Vec<OsString> = vec![
                "status".into(),
                "--porcelain".into(),
                "--no-renames".into(),
                untracked_arg.into(),
                "-z".into(),
            ];
            let out = git_read_bytes(args).unwrap_or_default();
            for entry in out.split(|&b| b == 0).filter(|s| s.len() > 3) {
                // worktree side has changes
                if matches!(entry[1], b'M' | b'T' | b'R')
                    && let Ok(path) = std::str::from_utf8(&entry[3..])
                {
                    let p = PathBuf::from(path);
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
        // The status left out paths that are not valid UTF-8, which hk cannot
        // name in a pathspec, so stash the whole worktree to set them aside
        let stash_everything =
            status.skipped_unstaged || (*env::HK_STASH_UNTRACKED && status.skipped_untracked);
        let files_count = files_to_stash.len();
        job.prop("files", &files_count);
        // TODO: if any intent_to_add files exist, run `git rm --cached -- <file>...` then `git add --intent-to-add -- <file>...` when unstashing
        // let intent_to_add = self.intent_to_add_files()?;
        // see https://github.com/pre-commit/pre-commit/blob/main/pre_commit/staged_files_only.py
        if files_to_stash.is_empty() && !stash_everything {
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
        let subset_opt: Option<&[PathBuf]> = if subset_vec.is_empty() || stash_everything {
            None
        } else {
            Some(&subset_vec[..])
        };
        self.stashed_paths = Some(files_to_stash);
        self.stash = self.push_stash(subset_opt, status)?;
        if self.stash.is_none() {
            self.stashed_paths = None;
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
        if let Some(ref ts) = tracked_subset
            && ts.is_empty()
        {
            if *env::HK_STASH_UNTRACKED {
                // No tracked files to stash, but we want to stash all untracked files
                // So do a full stash with --include-untracked (no pathspecs)
                return self.push_stash(None, status);
            } else {
                return Ok(None);
            }
        }
        // A path-limited stash decides what to reset from the HEAD-to-worktree diff,
        // even though the paths above were selected from the index-to-worktree diff.
        // If the former is empty (for example after `git update-index --chmod`), Git
        // creates a stash entry and resets the index before failing to reverse-apply
        // an empty patch. Avoid invoking Git in that destructive case.
        if let Some(ref ts) = tracked_subset
            && !ts.is_empty()
        {
            let diff_status = Command::new("git")
                .args([
                    "diff",
                    "--quiet",
                    "--no-ext-diff",
                    "--ignore-submodules",
                    "HEAD",
                    "--",
                ])
                .args(ts)
                .status()
                .wrap_err("failed to check whether stash pathspec has changes")?;
            match diff_status.code() {
                Some(0) => return Ok(None),
                Some(1) => {}
                code => {
                    return Err(eyre!(
                        "git diff failed while checking stash pathspec with exit code {code:?}"
                    ));
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
                run_git_stash(&cmd)?;
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
                        run_git_stash(&cmd)?;
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
            run_git_stash(&cmd)?;
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
                    if p.exists()
                        && let Ok(contents) = xx::file::read_to_string(&p)
                    {
                        wt_map.insert(p.clone(), contents);
                    }
                }
            }
        }
        self.saved_index = Some(entries);
        self.saved_worktree = Some(wt_map);
        Ok(())
    }

    /// Path of the most recent stash patch backup, if one was written.
    pub fn last_patch_path(&self) -> Option<&PathBuf> {
        self.last_patch_path.as_ref()
    }

    pub fn pop_stash(&mut self, should_stage: bool) -> Result<()> {
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
                        if let (Some(h), Some(gd)) = (parts.next(), parts.next())
                            && h == hash
                        {
                            found = Some(gd.to_string());
                            break;
                        }
                    }
                    found.unwrap_or_else(|| "stash@{0}".to_string())
                } else {
                    "stash@{0}".to_string()
                };

                // Track whether any file restoration failed so we can preserve the stash
                let mut restoration_failed = false;

                // What the stash set aside: how its worktree differs from the
                // index it recorded (^2), which includes deletions, mode changes
                // and files reverted to HEAD with a staged edit, plus its
                // untracked files (^3). `git stash show` compares with HEAD
                // instead and would miss the reverted files.
                let changes = stashed_changes(&stash_ref).unwrap_or_else(|err| {
                    warn!("failed to list the stashed files: {err:?}");
                    restoration_failed = true;
                    StashedChanges::default()
                });
                // Paths that are not valid UTF-8 are restored separately below
                let unnamed_paths = &changes.unnamed;
                // Paths that are staged as deletions in the current index. These must NOT be
                // restored to the worktree by the unstash loop for tracked files — the user
                // intentionally deleted them and the commit should preserve that deletion.
                // Note: a path can be staged-deleted AND still present on disk as an untracked
                // file (e.g., `git rm --cached`); the loop below preserves untracked restoration
                // by applying this skip only AFTER the is_untracked branch.
                let staged_deleted_set: std::collections::HashSet<PathBuf> =
                    git_read_paths(["diff", "--cached", "--name-only", "--diff-filter=D", "-z"])
                        .map(|(paths, _)| paths)
                        .unwrap_or_default()
                        .into_iter()
                        .collect();
                // Restore nothing if that would destroy what a step did; the
                // stash is kept instead
                let conflicts = changes
                    .restore_conflicts(&stash_ref, &staged_deleted_set)
                    .unwrap_or_else(|err| {
                        vec![format!(
                            "failed to compare the worktree with the stash: {err:?}"
                        )]
                    });
                for conflict in &conflicts {
                    warn!("not restoring the stash: {conflict}");
                }
                let restore = conflicts.is_empty();
                restoration_failed |= !restore;
                let unnamed_paths: &[Vec<u8>] = if restore { unnamed_paths } else { &[] };
                let stash_paths: Vec<PathBuf> = if restore {
                    changes
                        .modes
                        .keys()
                        .chain(changes.untracked.iter())
                        .cloned()
                        .collect()
                } else {
                    vec![]
                };
                // Paths restored to exactly their stashed state, which are
                // checked before the stash is dropped
                let mut snapshot_paths: BTreeSet<PathBuf> = BTreeSet::new();

                // When staging is disabled, fixer output remains in the isolated worktree.
                // Ask Git which hook files differ from the unchanged index so unchanged files
                // (especially large or binary ones) are not read eagerly.
                let fixer_worktree_paths: std::collections::HashSet<PathBuf> = if should_stage {
                    std::collections::HashSet::new()
                } else {
                    let candidate_paths: Vec<&PathBuf> = self
                        .saved_index
                        .as_deref()
                        .unwrap_or_default()
                        .iter()
                        .map(|(_, _, p)| p)
                        .filter(|p| stash_paths.contains(p))
                        .collect();
                    if candidate_paths.is_empty() {
                        std::collections::HashSet::new()
                    } else {
                        let mut args: Vec<OsString> = vec![
                            "diff".into(),
                            "--name-only".into(),
                            "-z".into(),
                            "--".into(),
                        ];
                        args.extend(
                            candidate_paths
                                .iter()
                                .map(|p| OsString::from(p.as_os_str())),
                        );
                        git_read(args)?
                            .split('\0')
                            .filter(|s| !s.is_empty())
                            .map(PathBuf::from)
                            .collect()
                    }
                };

                // Build a map of CURRENT index (post-step) entries to re-stage Fixer blobs.
                // Only include files that are actually staged-changed to avoid treating unrelated
                // tracked files (e.g., lockfiles) as fixers and pulling their contents into memory.
                let mut fixer_map: std::collections::HashMap<PathBuf, (u32, String)> =
                    std::collections::HashMap::new();
                // Determine the set of paths with staged changes (index differs from HEAD)
                let staged_changed_set: std::collections::HashSet<PathBuf> =
                    git_read_paths(["diff", "--name-only", "--cached", "-z"])
                        .map(|(paths, _)| paths)
                        .unwrap_or_default()
                        .into_iter()
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
                                if !oid.is_empty()
                                    && staged_changed_set.contains(&path_buf)
                                    && let Ok(mode_num) = u32::from_str_radix(mode, 8)
                                {
                                    fixer_map.insert(path_buf, (mode_num, oid.to_string()));
                                }
                            }
                        }
                    }
                }

                // Avoid excessive memory usage on very large files by short-circuiting
                // the merge logic when no fixer output exists for the path.
                const LARGE_STASH_FILE_BYTES: usize = 1_000_000; // 1 MiB

                let patch_hint = self
                    .last_patch_path()
                    .map(|p| format!("; stashed edits are backed up at {}", p.display()))
                    .unwrap_or_default();

                for p in stash_paths.iter() {
                    let path = PathBuf::from(p);
                    let path_str = p.to_string_lossy();

                    // Untracked files (in stash^3) are restored as they are
                    if changes.untracked.contains(&path) {
                        debug!(
                            "manual-unstash: restoring untracked file from stash^3 path={}",
                            display_path(&path)
                        );
                        match restore_stashed_path(
                            &stash_ref,
                            path.as_os_str(),
                            changes.entry(&path),
                        ) {
                            Ok(()) => {
                                snapshot_paths.insert(path);
                            }
                            Err(err) => {
                                warn!(
                                    "failed to restore untracked file {} from stash: {err:?}",
                                    display_path(&path)
                                );
                                restoration_failed = true;
                            }
                        }
                        continue;
                    }

                    // If this path is staged as a deletion in the current index, the user
                    // intentionally removed it — do not restore the tracked worktree blob.
                    // (Untracked variants — `git rm --cached` — are handled above.)
                    if staged_deleted_set.contains(&path) {
                        debug!(
                            "manual-unstash: skipping staged-deleted path={}",
                            display_path(&path)
                        );
                        continue;
                    }

                    let has_fixer = if should_stage {
                        fixer_map.contains_key(&path)
                    } else {
                        fixer_worktree_paths.contains(&path)
                    };
                    let (old_mode, new_mode) = changes.modes[&path];
                    // A deletion, symlink or type change has no contents to
                    // merge, so the stashed state wins over any fixer output
                    if new_mode == 0 || is_symlink_mode(new_mode) || is_symlink_mode(old_mode) {
                        if has_fixer {
                            warn!(
                                "{} was deleted or is a symlink in the stashed worktree; restoring that instead of the fixer output{}",
                                display_path(&path),
                                patch_hint
                            );
                        }
                        match restore_stashed_path(
                            &stash_ref,
                            path.as_os_str(),
                            changes.entry(&path),
                        ) {
                            Ok(()) => {
                                snapshot_paths.insert(path);
                            }
                            Err(err) => {
                                warn!(
                                    "failed to restore {} from stash: {err:?}",
                                    display_path(&path)
                                );
                                restoration_failed = true;
                            }
                        }
                        continue;
                    }
                    // Contents are written in place below, which keeps this mode
                    if let Err(err) = set_file_mode(&path, new_mode) {
                        warn!(
                            "failed to restore the mode of {}: {err:?}",
                            display_path(&path)
                        );
                        restoration_failed = true;
                    }
                    if !has_fixer {
                        snapshot_paths.insert(path.clone());
                    }
                    let work_ref = format!("{}:{}", stash_ref, path_str);
                    let work_size = git_cmd_silent(["cat-file", "-s", &work_ref])
                        .read()
                        .ok()
                        .and_then(|size| size.trim().parse::<usize>().ok());
                    if work_size.unwrap_or(0) >= LARGE_STASH_FILE_BYTES && !has_fixer {
                        debug!(
                            "manual-unstash: large file without fixer; restoring worktree snapshot directly path={} size={}",
                            display_path(&path),
                            work_size.unwrap_or(0)
                        );
                        if let Ok(bytes) = git_read_bytes(["cat-file", "-p", &work_ref]) {
                            if let Err(err) = xx::file::write(&path, &bytes) {
                                warn!(
                                    "failed to write large worktree snapshot for {}: {err:?}",
                                    display_path(&path)
                                );
                                restoration_failed = true;
                            }
                        } else {
                            warn!(
                                "failed to read large worktree snapshot for {}",
                                display_path(&path)
                            );
                            restoration_failed = true;
                        }
                        continue;
                    }

                    let fixer_worktree = if !should_stage && has_fixer {
                        if std::fs::symlink_metadata(&path)
                            .is_ok_and(|metadata| metadata.file_type().is_symlink())
                        {
                            warn!(
                                "fixer replaced {} with a symlink; preserving the symlink instead of conflicting stashed edits{}",
                                display_path(&path),
                                patch_hint
                            );
                            continue;
                        }
                        match std::fs::read(&path) {
                            Ok(contents) => match String::from_utf8(contents) {
                                Ok(contents) => Some(contents),
                                Err(err) => {
                                    warn!(
                                        "fixer wrote binary content to {}; preserving it instead of conflicting stashed edits{}",
                                        display_path(&path),
                                        patch_hint
                                    );
                                    if let Err(write_err) = xx::file::write(&path, err.as_bytes()) {
                                        warn!(
                                            "failed to preserve binary fixer output for {}: {write_err:?}",
                                            display_path(&path)
                                        );
                                        restoration_failed = true;
                                    }
                                    continue;
                                }
                            },
                            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                                warn!(
                                    "fixer deleted {}; preserving the deletion instead of conflicting stashed edits{}",
                                    display_path(&path),
                                    patch_hint
                                );
                                continue;
                            }
                            Err(err) => {
                                warn!(
                                    "failed to read fixer output for {}: {err:?}",
                                    display_path(&path)
                                );
                                restoration_failed = true;
                                continue;
                            }
                        }
                    } else {
                        None
                    };

                    // Detect binary files: try reading the worktree blob as UTF-8.
                    // If it fails, restore using raw bytes and skip text merging entirely.
                    let work_bytes = git_read_bytes(["cat-file", "-p", &work_ref]).ok();
                    let is_binary = work_bytes
                        .as_ref()
                        .is_some_and(|b| std::str::from_utf8(b).is_err());

                    if is_binary {
                        if !should_stage && fixer_worktree.is_some() {
                            warn!(
                                "text fixer output for {} cannot be merged with binary stashed edits; preserving the fixer output{}",
                                display_path(&path),
                                patch_hint
                            );
                            continue;
                        }
                        debug!(
                            "manual-unstash: binary file detected; restoring worktree snapshot directly path={}",
                            display_path(&path),
                        );
                        // SAFETY: is_binary is only true when work_bytes is Some (via is_some_and)
                        let bytes = work_bytes.unwrap();
                        if let Err(err) = xx::file::write(&path, &bytes) {
                            warn!(
                                "failed to write binary worktree snapshot for {}: {err:?}",
                                display_path(&path)
                            );
                            restoration_failed = true;
                        }
                        continue;
                    }

                    // Worktree content and Base (HEAD at stash time) from stash
                    // Prefer saved worktree snapshot captured before stashing; fallback to stash blob
                    // Prefer saved worktree snapshot; fall back to already-fetched work_bytes
                    // (which we know is valid UTF-8 since is_binary was false).
                    let work_pre = if let Some(map) = &self.saved_worktree {
                        map.get(&path).cloned()
                    } else {
                        None
                    }
                    .or_else(|| work_bytes.and_then(|b| String::from_utf8(b).ok()));
                    // Parent ^1 of the stash commit points to the HEAD commit at stash time
                    let base_pre =
                        git_read_raw(["cat-file", "-p", &format!("{}^1:{}", stash_ref, path_str)])
                            .ok();
                    // Parent ^2 is the index at stash time. Use this to detect whether the path had
                    // any unstaged changes then (worktree vs index).
                    let index_pre =
                        git_read_raw(["cat-file", "-p", &format!("{}^2:{}", stash_ref, path_str)])
                            .ok();
                    // Fixer content comes from the index when fixes were staged, or from the
                    // isolated post-step worktree when staging was disabled.
                    let fixer = if should_stage {
                        fixer_map
                            .get(&path)
                            .and_then(|(_, oid)| git_read_raw(["cat-file", "-p", oid]).ok())
                    } else {
                        fixer_worktree
                    };

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
                        // If that fails, allow a single trailing newline discrepancy, but only
                        // when the worktree is exactly the index minus its trailing newline.
                        // A non-empty remainder here means the LAST LINE was edited (e.g.
                        // i="…l3\n", w="…l3 edited\n"), which is not a pure tail insertion;
                        // treating it as one would split the edit onto a new line. Leave those
                        // to the regular three-way merge instead.
                        if tail_opt.is_none() && i.ends_with('\n') {
                            tail_opt = w
                                .strip_prefix(&i[..i.len().saturating_sub(1)])
                                .filter(|tail| tail.is_empty());
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
                    if newline_only_change
                        && let (Some(w), Some(i)) = (work_pre.as_deref(), index_pre.as_deref())
                    {
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

                    // If there were no unstaged changes at stash time for this path
                    // (worktree identical to index), prefer writing the fixer result to the worktree
                    // so that files formatted by fixers (e.g., Prettier) appear in the worktree post-commit.
                    if !newline_only_change
                        && let (Some(wc), Some(ic), Some(fc)) =
                            (work_pre.as_ref(), index_pre.as_ref(), fixer.as_ref())
                        && wc == ic
                    {
                        merged = fc.clone();
                    }

                    // Determine which side the merged result matches
                    let mut chosen = "mixed";
                    if let Some(w) = work_pre.as_deref()
                        && merged == w
                    {
                        chosen = "worktree";
                    }
                    if chosen == "mixed"
                        && let Some(f) = fixer.as_deref()
                        && merged == f
                    {
                        chosen = "fixer";
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
                    } else if should_stage && let Some((mode, oid)) = fixer_map.get(&path) {
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
                // hk gave no step the paths that are not valid UTF-8, so restore
                // their stashed state as it is
                for name in unnamed_paths {
                    let path = path_from_raw(name);
                    match restore_stashed_path(&stash_ref, path.as_os_str(), changes.entry(&path)) {
                        Ok(()) => {
                            snapshot_paths.insert(path);
                        }
                        Err(err) => {
                            warn!(
                                "failed to restore {:?} from stash: {err:?}",
                                String::from_utf8_lossy(name)
                            );
                            restoration_failed = true;
                        }
                    }
                }
                // Before dropping the stash, check that every path restored to
                // its stashed state matches it
                if !restoration_failed {
                    match unrestored_paths(&stash_ref, &changes, &snapshot_paths) {
                        Ok(unrestored) if unrestored.is_empty() => {}
                        Ok(unrestored) => {
                            warn!(
                                "restoring the stash left {} different from the stash",
                                unrestored.iter().map(display_path).join(", ")
                            );
                            restoration_failed = true;
                        }
                        Err(err) => {
                            warn!("failed to check the restored files: {err:?}");
                            restoration_failed = true;
                        }
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
        self.stashed_paths = None;
        Ok(())
    }

    /// Stages exactly `paths`, taken literally rather than as pathspecs.
    ///
    /// Always runs `git add`, even with libgit2: writing the index re-checks
    /// recently staged entries against the worktree, and other steps may still
    /// be writing those files. Git smudges an entry whose file changes while it
    /// reads it; libgit2 fails the whole write instead.
    pub fn add(&self, paths: &[PathBuf]) -> Result<()> {
        trace!("adding files: {:?}", paths);
        if paths.is_empty() {
            return Ok(());
        }
        // Pass the paths on stdin: a large fixer's files can exceed the
        // command-line limit.
        let mut pathspecs = Vec::new();
        for p in paths {
            pathspecs.extend_from_slice(b":(literal)");
            pathspecs.extend_from_slice(p.as_os_str().as_encoded_bytes());
            pathspecs.push(0);
        }
        git_cmd(["add", "--pathspec-from-file=-", "--pathspec-file-nul"])
            .stdin_bytes(pathspecs)
            .run()?;
        Ok(())
    }

    pub fn files_between_refs(&self, from_ref: &str, to_ref: Option<&str>) -> Result<Vec<PathBuf>> {
        let to_ref = to_ref.unwrap_or("HEAD");
        if let Some(repo) = &self.repo {
            let to_obj = repo
                .revparse_single(to_ref)
                .wrap_err(format!("Failed to parse reference: {to_ref}"))?;

            let to_commit = to_obj
                .peel_to_commit()
                .wrap_err(format!("Failed to get commit for reference: {to_ref}"))?;
            let to_tree = to_commit
                .tree()
                .wrap_err(format!("Failed to get tree for reference: {to_ref}"))?;

            let from_obj = match repo.revparse_single(from_ref) {
                Ok(from_obj) => from_obj,
                // Unresolvable from-ref (e.g. first push, `default_branch()` returned the literal
                // "origin/HEAD"): diff against the empty tree so all pushed files are linted.
                // Passing `None` as the old tree diffs against empty without materializing an
                // empty-tree object in the ODB, keeping this query side-effect free.
                Err(err) if err.code() == ErrorCode::NotFound => {
                    debug!("could not resolve from-ref '{from_ref}'; diffing against empty tree");
                    let diff = repo
                        .diff_tree_to_tree(None, Some(&to_tree), None)
                        .wrap_err("Failed to diff against empty tree")?;
                    return collect_existing_paths_from_diff(diff);
                }
                Err(err) => {
                    return Err(err).wrap_err(format!("Failed to resolve from-ref: {from_ref}"));
                }
            };
            let from_commit = from_obj
                .peel_to_commit()
                .wrap_err(format!("Failed to get commit for reference: {from_ref}"))?;

            // Prefer merge-base semantics (`from...to`). In shallow clones the
            // merge base can be missing, so fall back to a direct `from..to`
            // diff instead of failing the whole run.
            let diff = match repo.merge_base(from_commit.id(), to_commit.id()) {
                Ok(merge_base) => {
                    let merge_base_obj = repo
                        .find_object(merge_base, None)
                        .wrap_err("Failed to find merge base object")?;
                    let merge_base_tree = merge_base_obj
                        .peel_to_tree()
                        .wrap_err("Failed to get tree for merge base")?;
                    repo.diff_tree_to_tree(Some(&merge_base_tree), Some(&to_tree), None)
                        .wrap_err("Failed to get diff between references")?
                }
                Err(err) if err.code() == ErrorCode::NotFound => {
                    if let Some(merge_base) = git_merge_base(from_ref, to_ref)? {
                        let merge_base_obj = repo
                            .find_object(
                                git2::Oid::from_str(merge_base.trim())
                                    .wrap_err("Failed to parse git merge-base output")?,
                                None,
                            )
                            .wrap_err("Failed to find merge base object")?;
                        let merge_base_tree = merge_base_obj
                            .peel_to_tree()
                            .wrap_err("Failed to get tree for merge base")?;
                        repo.diff_tree_to_tree(Some(&merge_base_tree), Some(&to_tree), None)
                            .wrap_err("Failed to get diff between references")?
                    } else {
                        let from_tree = from_commit
                            .tree()
                            .wrap_err(format!("Failed to get tree for reference: {from_ref}"))?;
                        repo.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)
                            .wrap_err("Failed to get fallback diff between references")?
                    }
                }
                Err(err) => return Err(err).wrap_err("Failed to find merge base"),
            };

            collect_existing_paths_from_diff(diff)
        } else if git_rev_exists(from_ref)? {
            let range = match git_merge_base(from_ref, to_ref)? {
                Some(merge_base) => format!("{}..{}", merge_base.trim(), to_ref),
                None => format!("{from_ref}..{to_ref}"),
            };

            let output = git_read([
                "diff",
                "-z",
                "--name-only",
                "--diff-filter=ACMRTUXB",
                "--end-of-options",
                range.as_str(),
            ])?;
            Ok(output
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect())
        } else {
            // No resolvable base: lint every file at `to_ref`. `ls-tree` is
            // object-format agnostic, unlike a hard-coded empty-tree hash.
            debug!("could not resolve from-ref '{from_ref}'; listing all files at {to_ref}");
            let output = git_read([
                "ls-tree",
                "-z",
                "-r",
                "--name-only",
                "--end-of-options",
                to_ref,
            ])?;
            Ok(output
                .split('\0')
                .filter(|p| !p.is_empty())
                .map(PathBuf::from)
                .collect())
        }
    }
}

fn collect_existing_paths_from_diff(diff: Diff<'_>) -> Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    diff.foreach(
        &mut |diff_delta, _| {
            if diff_delta.status() == git2::Delta::Deleted {
                return true;
            }
            if let Some(path) = diff_delta.new_file().path() {
                let path_buf = PathBuf::from(path);
                if path_buf.exists() {
                    files.insert(path_buf);
                }
            }
            true
        },
        None,
        None,
        None,
    )
    .wrap_err("Failed to process diff")?;

    Ok(files.into_iter().collect())
}

fn git_rev_exists(rev: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", "--end-of-options", rev])
        .output()
        .wrap_err("Failed to run git rev-parse")?;

    match output.status.code() {
        // 0: rev resolves. 1: rev is absent (with --quiet). Anything else (or a
        // spawn failure above) is a real error, not evidence the rev is missing.
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(eyre!(
            "Failed to check rev '{rev}': {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

fn git_merge_base(from_ref: &str, to_ref: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["merge-base", "--end-of-options", from_ref, to_ref])
        .output()
        .wrap_err("Failed to run git merge-base")?;

    match output.status.code() {
        Some(0) => String::from_utf8(output.stdout)
            .map(Some)
            .map_err(|err| eyre!("git merge-base output is not valid UTF-8: {err}")),
        Some(1) => Ok(None),
        _ => Err(eyre!(
            "Failed to find merge base: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
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
    /// Whether the status left out a path with changes in the worktree because
    /// it is not valid UTF-8, so that the lists above miss unstaged changes
    #[serde(skip)]
    pub skipped_unstaged: bool,
    /// Whether the status left out an untracked path because it is not valid
    /// UTF-8
    #[serde(skip)]
    pub skipped_untracked: bool,
}

impl GitStatus {
    /// Whether the worktree has changes that stashing would set aside,
    /// including untracked files when `include_untracked` is set.
    pub fn has_unstaged_changes(&self, include_untracked: bool) -> bool {
        !self.unstaged_files.is_empty()
            || self.skipped_unstaged
            || (include_untracked && (!self.untracked_files.is_empty() || self.skipped_untracked))
    }

    fn set_skipped(&mut self, skipped: &SkippedPaths) {
        self.skipped_unstaged |= skipped.unstaged;
        self.skipped_untracked |= skipped.untracked;
    }

    /// Adds the entries of a status of other paths.
    fn extend(&mut self, other: GitStatus) {
        self.skipped_unstaged |= other.skipped_unstaged;
        self.skipped_untracked |= other.skipped_untracked;
        self.unstaged_files.extend(other.unstaged_files);
        self.staged_files.extend(other.staged_files);
        self.untracked_files.extend(other.untracked_files);
        self.modified_files.extend(other.modified_files);
        self.staged_added_files.extend(other.staged_added_files);
        self.staged_modified_files
            .extend(other.staged_modified_files);
        self.staged_deleted_files.extend(other.staged_deleted_files);
        self.staged_renamed_files.extend(other.staged_renamed_files);
        self.staged_copied_files.extend(other.staged_copied_files);
        self.unstaged_modified_files
            .extend(other.unstaged_modified_files);
        self.unstaged_deleted_files
            .extend(other.unstaged_deleted_files);
        self.unstaged_renamed_files
            .extend(other.unstaged_renamed_files);
    }

    /// Classifies entries of `git status --porcelain=v2`.
    fn from_entries(entries: Vec<StatusEntry>) -> Self {
        let mut status = Self::default();
        for entry in entries {
            let StatusEntry {
                index,
                worktree,
                path,
                ..
            } = entry;
            let exists = path_exists(&path);
            let is_modified = |c: u8| matches!(c, b'M' | b'T' | b'A' | b'R' | b'C');

            // Only consider staged files that still exist in the worktree to avoid AD cases
            if is_modified(index) && worktree != b'D' && exists {
                status.staged_files.insert(path.clone());
            }
            // Classify staged/index status
            match index {
                b'A' => {
                    status.staged_added_files.insert(path.clone());
                }
                b'M' | b'T' => {
                    status.staged_modified_files.insert(path.clone());
                }
                b'D' => {
                    status.staged_deleted_files.insert(path.clone());
                }
                b'R' => {
                    status.staged_renamed_files.insert(path.clone());
                }
                b'C' => {
                    status.staged_copied_files.insert(path.clone());
                }
                _ => {}
            }
            // Unstaged files include actual worktree changes (not untracked files)
            if is_modified(worktree) && exists {
                status.unstaged_files.insert(path.clone());
            }
            if worktree == b'?' && exists {
                status.untracked_files.insert(path.clone());
            }
            // Track modified files only if the path exists
            if (is_modified(index) || is_modified(worktree)) && exists {
                status.modified_files.insert(path.clone());
            }
            // Classify workdir status
            match worktree {
                b'M' | b'T' => {
                    status.unstaged_modified_files.insert(path);
                }
                b'D' => {
                    status.unstaged_deleted_files.insert(path);
                }
                b'R' => {
                    status.unstaged_renamed_files.insert(path);
                }
                _ => {}
            }
        }
        status
    }

    /// Classifies entries of `git status --porcelain=v2 --renames` the way
    /// [`Git::read_status`] classifies libgit2's statuses, so that a status
    /// reads the same whichever of the two produced it.
    ///
    /// Adds original paths it leaves out because they are not valid UTF-8 to
    /// `skipped`.
    fn from_entries_libgit2(entries: Vec<StatusEntry>, skipped: &mut SkippedPaths) -> Self {
        let mut status = Self::default();
        for entry in entries {
            let path = entry.path;
            let exists = path_exists(&path);
            // libgit2 reports an unmerged entry as conflicted on both sides,
            // without classifying it
            if entry.unmerged {
                if exists {
                    status.staged_files.insert(path.clone());
                    status.unstaged_files.insert(path);
                }
                continue;
            }
            let (index, worktree) = match (entry.index, entry.worktree) {
                // libgit2 reports an intent-to-add entry as added to the index
                // with empty contents, which the worktree then modifies. git
                // pairs an intent-to-add entry with a file deleted from the
                // worktree as a worktree rename (`.R`), which libgit2 reports
                // as that file deleted. Only such pairs are worktree renames:
                // git renames between index entries, and an intent-to-add
                // entry is the only one whose file is new in the worktree, so
                // a file moved without `git add -N` is `.D` plus untracked.
                (b' ', b'A' | b'R') => {
                    if entry.worktree == b'R'
                        && let Some(orig_path) = &entry.orig_path
                        && let Some(orig_path) = utf8_path(orig_path, skipped)
                    {
                        status.unstaged_deleted_files.insert(orig_path);
                    }
                    let empty =
                        std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && m.len() == 0);
                    (b'A', if empty { b' ' } else { b'M' })
                }
                other => other,
            };
            // HEAD to index
            if !matches!(index, b' ' | b'?') {
                match index {
                    // libgit2 detects renames but not copies
                    b'A' | b'C' => {
                        status.staged_added_files.insert(path.clone());
                    }
                    b'M' | b'T' => {
                        status.staged_modified_files.insert(path.clone());
                    }
                    b'D' => {
                        status.staged_deleted_files.insert(path.clone());
                    }
                    b'R' => {
                        status.staged_renamed_files.insert(path.clone());
                        // libgit2 also flags a rename that changed the contents
                        // as modified
                        if entry.rename_modified {
                            status.staged_modified_files.insert(path.clone());
                        }
                    }
                    _ => {}
                }
                if exists {
                    status.staged_files.insert(path.clone());
                }
            }
            // Index to worktree
            match worktree {
                b'?' => {
                    status.untracked_files.insert(path.clone());
                }
                b'M' | b'T' => {
                    status.modified_files.insert(path.clone());
                    status.unstaged_modified_files.insert(path.clone());
                }
                b'D' => {
                    status.unstaged_deleted_files.insert(path.clone());
                }
                _ => {}
            }
            if exists && !matches!(worktree, b' ' | b'?') {
                status.unstaged_files.insert(path);
            }
        }
        status
    }
}

/// One entry of `git status --porcelain=v2 -z`.
struct StatusEntry {
    /// The index compared with HEAD (`X`), `b' '` when unchanged and `b'?'`
    /// when untracked
    index: u8,
    /// The worktree compared with the index (`Y`), `b' '` when unchanged and
    /// `b'?'` when untracked
    worktree: u8,
    /// Whether the entry is unmerged
    unmerged: bool,
    /// Whether a staged rename or copy also changed the contents
    rename_modified: bool,
    /// The path, the new one for a rename or copy
    path: PathBuf,
    /// The original path of a rename or copy, as git printed it: it is
    /// checked only where it is used
    orig_path: Option<Vec<u8>>,
}

/// Paths a status read left out because they are not valid UTF-8, which hk
/// cannot pass on to steps and templates.
#[derive(Debug, Default)]
struct SkippedPaths {
    /// The paths, shown lossily
    names: Vec<String>,
    /// Whether any of them has changes in the worktree
    unstaged: bool,
    /// Whether any of them is untracked
    untracked: bool,
}

/// Parses `git status --porcelain=v2 -z`, leaving out entries whose path is
/// not valid UTF-8 and recording them in `skipped`.
fn parse_porcelain_status(output: &[u8], skipped: &mut SkippedPaths) -> Result<Vec<StatusEntry>> {
    let malformed = |line: &[u8]| {
        eyre!(
            "unexpected git status entry: {}",
            String::from_utf8_lossy(line)
        )
    };
    let mut entries = Vec::new();
    let mut lines = output.split(|&b| b == 0);
    while let Some(line) = lines.next() {
        // Fields before the path, which may itself contain spaces
        let field_count = match line.first() {
            Some(b'1') => 8,
            Some(b'2') => 9,
            Some(b'u') => 10,
            Some(b'?') => 1,
            // Headers and ignored files, which are not requested
            Some(b'#' | b'!') => continue,
            None if line.is_empty() => continue,
            _ => return Err(malformed(line)),
        };
        let fields = line.splitn(field_count + 1, |&b| b == b' ').collect_vec();
        if fields.len() != field_count + 1 || fields[0].len() != 1 {
            return Err(malformed(line));
        }
        let (index, worktree) = match (fields[0], fields[1]) {
            (b"?", _) => (b'?', b'?'),
            (_, [x, y]) => {
                let unchanged = |c: u8| if c == b'.' { b' ' } else { c };
                (unchanged(*x), unchanged(*y))
            }
            _ => return Err(malformed(line)),
        };
        let orig_path = if fields[0] == b"2" {
            // The original path follows as a separate NUL-terminated field
            let orig = lines.next().ok_or_else(|| malformed(line))?;
            Some(orig.to_vec())
        } else {
            None
        };
        let Some(path) = utf8_path(fields[field_count], skipped) else {
            // Stashing must still set the skipped file's changes aside
            skipped.unstaged |= !matches!(worktree, b' ' | b'?');
            skipped.untracked |= worktree == b'?';
            continue;
        };
        entries.push(StatusEntry {
            index,
            worktree,
            unmerged: fields[0] == b"u",
            // The HEAD and index object names of a rename or copy differ
            rename_modified: fields[0] == b"2" && fields[6] != fields[7],
            path,
            orig_path,
        });
    }
    Ok(entries)
}

/// A path from git's status output, or `None` after adding it to `skipped`
/// when it is not valid UTF-8.
fn utf8_path(bytes: &[u8], skipped: &mut SkippedPaths) -> Option<PathBuf> {
    match std::str::from_utf8(bytes) {
        Ok(path) => Some(PathBuf::from(path)),
        Err(_) => {
            skipped
                .names
                .push(String::from_utf8_lossy(bytes).into_owned());
            None
        }
    }
}

/// Warns that a status read left out `skipped`, so that files hk does not
/// check are not left out silently.
fn warn_skipped_paths(skipped: &SkippedPaths) {
    if !skipped.names.is_empty() {
        warn!(
            "skipped {} because hk cannot handle paths that are not valid UTF-8",
            skipped.names.iter().map(|p| format!("{p:?}")).join(", ")
        );
    }
}

/// Whether `path` exists, counting broken symlinks, which
/// [`std::path::Path::exists`] does not.
fn path_exists(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn untracked_files_arg(include_untracked: bool) -> &'static str {
    if include_untracked {
        "--untracked-files=all"
    } else {
        "--untracked-files=no"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `git status --porcelain=v2 -z` output for files in `dir`: a header, a
    /// staged modification, staged renames with and without content changes,
    /// intent-to-add entries with and without contents, an intent-to-add
    /// entry that git pairs with a deleted file as a worktree rename, a
    /// conflict, a file added then deleted, and an untracked file.
    fn sample(dir: &std::path::Path) -> Vec<u8> {
        for (name, contents) in [
            ("staged.txt", "x"),
            ("renamed.txt", "x"),
            ("ita.txt", "x"),
            ("empty_ita.txt", ""),
            ("ita_moved.txt", "x"),
            ("conflict.txt", "x"),
            ("with space.txt", "x"),
        ] {
            std::fs::write(dir.join(name), contents).unwrap();
        }
        let d = dir.display();
        let (z, a, b) = ("0".repeat(40), "a".repeat(40), "b".repeat(40));
        [
            format!("# branch.oid {a}"),
            format!("1 M. N... 100644 100644 100644 {a} {b} {d}/staged.txt"),
            format!("2 R. N... 100644 100644 100644 {a} {b} R87 {d}/renamed.txt"),
            format!("{d}/old.txt"),
            format!("2 R. N... 100644 100644 100644 {a} {a} R100 {d}/moved.txt"),
            format!("{d}/old2.txt"),
            format!("1 .A N... 000000 000000 100644 {z} {z} {d}/ita.txt"),
            format!("1 .A N... 000000 000000 100644 {z} {z} {d}/empty_ita.txt"),
            format!("2 .R N... 100644 100644 100644 {a} {a} R100 {d}/ita_moved.txt"),
            format!("{d}/ita_source.txt"),
            format!("u UU N... 100644 100644 100644 100644 {a} {b} {a} {d}/conflict.txt"),
            format!("1 AD N... 000000 100644 000000 {z} {a} {d}/gone.txt"),
            format!("? {d}/with space.txt"),
        ]
        .iter()
        .map(|line| format!("{line}\0"))
        .collect::<String>()
        .into_bytes()
    }

    /// Entries of `output`, none of which may be skipped.
    fn parse(output: &[u8]) -> Vec<StatusEntry> {
        let mut skipped = SkippedPaths::default();
        let entries = parse_porcelain_status(output, &mut skipped).unwrap();
        assert!(skipped.names.is_empty(), "{skipped:?}");
        entries
    }

    /// The libgit2 classification of `entries`, none of which may be skipped.
    fn libgit2(entries: Vec<StatusEntry>) -> GitStatus {
        let mut skipped = SkippedPaths::default();
        let status = GitStatus::from_entries_libgit2(entries, &mut skipped);
        assert!(skipped.names.is_empty(), "{skipped:?}");
        status
    }

    fn paths(dir: &std::path::Path, names: &[&str]) -> BTreeSet<PathBuf> {
        names.iter().map(|name| dir.join(name)).collect()
    }

    #[test]
    fn test_porcelain_status_classified_like_libgit2() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let entries = parse(&sample(d));
        let status = libgit2(entries);
        let staged = ["staged.txt", "renamed.txt", "ita.txt", "empty_ita.txt"];
        let staged = [&staged[..], &["ita_moved.txt", "conflict.txt"]].concat();
        assert_eq!(status.staged_files, paths(d, &staged));
        let added = ["ita.txt", "empty_ita.txt", "ita_moved.txt", "gone.txt"];
        assert_eq!(status.staged_added_files, paths(d, &added));
        // libgit2 flags a rename that changed the contents as modified too
        assert_eq!(
            status.staged_modified_files,
            paths(d, &["staged.txt", "renamed.txt"])
        );
        assert_eq!(
            status.staged_renamed_files,
            paths(d, &["renamed.txt", "moved.txt"])
        );
        assert!(status.staged_deleted_files.is_empty());
        assert!(status.staged_copied_files.is_empty());
        assert_eq!(
            status.unstaged_files,
            paths(d, &["ita.txt", "ita_moved.txt", "conflict.txt"])
        );
        let modified = paths(d, &["ita.txt", "ita_moved.txt"]);
        assert_eq!(status.modified_files, modified);
        assert_eq!(status.unstaged_modified_files, modified);
        // libgit2 does not pair worktree renames: their source is deleted
        assert_eq!(
            status.unstaged_deleted_files,
            paths(d, &["ita_source.txt", "gone.txt"])
        );
        assert!(status.unstaged_renamed_files.is_empty());
        assert_eq!(status.untracked_files, paths(d, &["with space.txt"]));
    }

    #[test]
    fn test_porcelain_status_classified_like_git() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let entries = parse(&sample(d));
        let status = GitStatus::from_entries(entries);
        assert_eq!(
            status.staged_files,
            paths(d, &["staged.txt", "renamed.txt"])
        );
        assert_eq!(status.staged_added_files, paths(d, &["gone.txt"]));
        assert_eq!(status.staged_modified_files, paths(d, &["staged.txt"]));
        assert_eq!(
            status.staged_renamed_files,
            paths(d, &["renamed.txt", "moved.txt"])
        );
        assert_eq!(
            status.unstaged_files,
            paths(d, &["ita.txt", "empty_ita.txt", "ita_moved.txt"])
        );
        let modified = ["staged.txt", "renamed.txt", "ita.txt", "empty_ita.txt"];
        let modified = [&modified[..], &["ita_moved.txt"]].concat();
        assert_eq!(status.modified_files, paths(d, &modified));
        assert_eq!(status.unstaged_deleted_files, paths(d, &["gone.txt"]));
        assert_eq!(status.unstaged_renamed_files, paths(d, &["ita_moved.txt"]));
        assert_eq!(status.untracked_files, paths(d, &["with space.txt"]));
    }

    #[test]
    fn test_porcelain_status_plain_move_stages_nothing() {
        // `mv a.txt b.txt` without `git add -N`, as git reports it whatever
        // its rename settings
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        std::fs::write(d.join("b.txt"), "x").unwrap();
        let a = "a".repeat(40);
        let output = format!(
            "1 .D N... 100644 100644 000000 {a} {a} {}/a.txt\0? {}/b.txt\0",
            d.display(),
            d.display()
        );
        let entries = || parse(output.as_bytes());
        for status in [libgit2(entries()), GitStatus::from_entries(entries())] {
            assert!(status.staged_files.is_empty());
            assert!(status.staged_added_files.is_empty());
            assert!(status.unstaged_files.is_empty());
            assert_eq!(status.unstaged_deleted_files, paths(d, &["a.txt"]));
            assert_eq!(status.untracked_files, paths(d, &["b.txt"]));
        }
    }

    #[test]
    fn test_porcelain_status_skips_paths_that_are_not_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().display();
        std::fs::write(dir.path().join("ok.txt"), "x").unwrap();
        let a = "a".repeat(40);
        let output = [
            format!("1 A. N... 000000 100644 100644 {a} {a} {d}/bad").as_bytes(),
            b"\xff.txt\0? bad\xfe.txt\0",
            format!("2 R. N... 100644 100644 100644 {a} {a} R100 {d}/bad").as_bytes(),
            b"\xfd.txt\0old.txt\0",
            format!("1 A. N... 000000 100644 100644 {a} {a} {d}/ok.txt\0").as_bytes(),
        ]
        .concat();
        let mut skipped = SkippedPaths::default();
        let entries = parse_porcelain_status(&output, &mut skipped).unwrap();
        assert_eq!(
            skipped.names,
            [
                format!("{d}/bad\u{fffd}.txt"),
                "bad\u{fffd}.txt".to_string(),
                format!("{d}/bad\u{fffd}.txt"),
            ]
        );
        // Only the untracked one needs stashing: the others are staged
        assert!(!skipped.unstaged);
        assert!(skipped.untracked);
        let mut status = GitStatus::from_entries(entries);
        status.set_skipped(&skipped);
        assert_eq!(status.staged_files, paths(dir.path(), &["ok.txt"]));
        assert!(status.untracked_files.is_empty());
        assert!(status.unstaged_files.is_empty());
        // A skipped untracked file counts as a change to stash only with
        // untracked files
        assert!(status.has_unstaged_changes(true));
        assert!(!status.has_unstaged_changes(false));

        // A skipped file changed in the worktree always counts
        let mut skipped = SkippedPaths::default();
        let modified = format!("1 .M N... 100644 100644 100644 {a} {a} {d}/bad");
        let modified = [modified.as_bytes(), b"\xfc.txt\0"].concat();
        assert!(
            parse_porcelain_status(&modified, &mut skipped)
                .unwrap()
                .is_empty()
        );
        assert!(skipped.unstaged);
        assert!(!skipped.untracked);
        let mut status = GitStatus::default();
        status.set_skipped(&skipped);
        assert!(status.has_unstaged_changes(false));
    }

    #[test]
    fn test_porcelain_status_checks_rename_sources_only_where_used() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().display();
        std::fs::write(dir.path().join("new.txt"), "x").unwrap();
        let a = "a".repeat(40);
        let staged = format!("2 R. N... 100644 100644 100644 {a} {a} R100 {d}/new.txt\0old");
        let staged = [staged.as_bytes(), b"\xfd.txt\0"].concat();
        let worktree = format!("2 .R N... 100644 100644 100644 {a} {a} R100 {d}/new.txt\0old");
        let worktree = [worktree.as_bytes(), b"\xfd.txt\0"].concat();

        // Neither classification uses the source of a staged rename
        let status = libgit2(parse(&staged));
        assert_eq!(status.staged_renamed_files, paths(dir.path(), &["new.txt"]));
        let status = GitStatus::from_entries(parse(&worktree));
        assert_eq!(
            status.unstaged_renamed_files,
            paths(dir.path(), &["new.txt"])
        );
        // The libgit2 classification lists the source of a worktree rename
        // as deleted, so it skips one that is not valid UTF-8 and keeps the
        // destination
        let mut skipped = SkippedPaths::default();
        let status = GitStatus::from_entries_libgit2(parse(&worktree), &mut skipped);
        assert_eq!(skipped.names, ["old\u{fffd}.txt"]);
        assert!(status.unstaged_deleted_files.is_empty());
        assert_eq!(status.staged_added_files, paths(dir.path(), &["new.txt"]));
    }

    #[test]
    fn test_porcelain_status_rejects_malformed_entries() {
        let parse = |output: &[u8]| parse_porcelain_status(output, &mut SkippedPaths::default());
        assert!(parse(b"3 what\0").is_err());
        assert!(parse(b"1 M. N... 100644\0").is_err());
        let a = "a".repeat(40);
        // A rename without its original path
        let renamed = format!("2 R. N... 100644 100644 100644 {a} {a} R100 new.txt");
        assert!(parse(renamed.as_bytes()).is_err());
        // Malformed even though the path is not valid UTF-8
        assert!(parse(b"1 M. bad\xff.txt\0").is_err());
    }
}
