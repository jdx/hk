use crate::{Result, config::Config, env, git_util};
use eyre::bail;
use log::{info, warn};
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Hook events installed by default for `hk install --global` when no project
/// config is available to enumerate a more specific set.
const CORE_GLOBAL_EVENTS: &[&str] = &["commit-msg", "pre-commit", "pre-push", "prepare-commit-msg"];

/// Set up git hooks to run hk.
///
/// The recommended setup is `hk install --global`, which installs hooks
/// once into the user's `~/.gitconfig` so every repository on the machine
/// picks them up automatically. In a project without an `hk.pkl`, the
/// installed hook exits silently — no-op — so it's safe to enable
/// everywhere. Requires Git 2.54+.
///
/// Without `--global`, hooks are installed into the current repo only.
/// On Git 2.54+ this uses config-based hooks (`hook.<name>.command`),
/// which keeps `.git/hooks/` untouched and composes cleanly with other
/// hook managers. On older Git it falls back to writing script shims.
///
/// If hk is already configured globally (any `hook.hk-*` entry in
/// `~/.gitconfig`), the per-repo install is skipped — and any stale
/// local hooks are cleaned up — so the global install remains the
/// single source of truth and hk doesn't fire twice per event. Pass
/// `--force-local` to install local hooks anyway.
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "write")]
pub struct Install {
    /// Install local hooks even when hk is already configured globally
    /// (any `hook.hk-*` entry in `~/.gitconfig`). By default a per-repo
    /// install is skipped in that case to avoid hk firing twice per
    /// event. Not compatible with `--global`.
    #[usage(long, verbatim_doc_comment, conflicts = "--global")]
    force_local: bool,

    /// Recommended. Install at user level (~/.gitconfig) so every repo
    /// on this machine gets hk hooks. Requires Git 2.54 or newer. In
    /// repos without an `hk.pkl`, the installed hook is a silent no-op.
    #[usage(long, verbatim_doc_comment)]
    global: bool,

    /// Force using the legacy `.git/hooks/` script shims instead of Git
    /// 2.54+ config-based hooks. Not compatible with `--global`.
    #[usage(long, verbatim_doc_comment, conflicts = "--global")]
    legacy: bool,

    /// Run hooks through `mise x` so mise-managed tools are available
    /// without activating mise in the shell.
    ///
    /// Set HK_MISE=1 to make this the default behavior.
    #[usage(long, verbatim_doc_comment)]
    mise: bool,
}

impl Install {
    pub async fn run(&self) -> Result<()> {
        let use_mise = *env::HK_MISE || self.mise;

        if self.global {
            if !git_util::git_at_least(2, 54) {
                bail!(
                    "`hk install --global` requires Git 2.54+ (config-based hooks). Detected git version does not support this. Upgrade git, or install per-repo with `hk install`."
                );
            }
            let command = global_hook_command(use_mise)?;
            let events = global_hook_events()?;
            return install_global(&events, &command);
        }

        if !self.force_local && has_global_hk_hooks()? {
            let _config_lock = lock_local_config()?;
            // The global install is the single source of truth; clean up any
            // stale local install so it doesn't double-fire alongside global.
            let removed = remove_local_shims()? + remove_local_config_entries()?;
            if removed > 0 {
                info!(
                    "hk hooks already configured globally (~/.gitconfig); removed {removed} stale local hook(s) and did not install new ones. Pass `--force-local` to install per-repo hooks anyway."
                );
            } else {
                info!(
                    "hk hooks already configured globally (~/.gitconfig); skipping local install. Pass `--force-local` to install per-repo hooks anyway."
                );
            }
            return Ok(());
        }

        let command = local_hook_command(use_mise);
        let path_fallback = local_path_fallback(use_mise);
        let use_config_hooks = !self.legacy && git_util::git_at_least(2, 54);

        // Load and validate the project config before touching anything, so a
        // broken `hk.pkl` doesn't leave the repo with its prior hooks removed.
        let config = Config::get()?;
        let events = hook_events(&config);

        // Git locks each config write separately. Hold a separate lock across
        // the entire removal and installation so concurrent hk processes
        // cannot observe or overwrite each other's partial installs.
        let _config_lock = lock_local_config()?;
        // Clean up any prior installation so modes don't accumulate.
        let removed = remove_local_shims()? + remove_local_config_entries()?;

        if events.is_empty() {
            if removed > 0 {
                warn!(
                    "no hooks configured in hk.pkl — removed {removed} previously-installed hk hook(s) and did not install any new ones"
                );
            } else {
                warn!("no hooks configured in hk.pkl — nothing to install");
            }
            return Ok(());
        }

        if use_config_hooks {
            let result = install_local_config(&events, &command, path_fallback.as_deref());
            warn_if_global_overlap(&events);
            result
        } else {
            install_local_shims(&events, &command, path_fallback.as_deref())
        }
    }
}

/// Returns true if any `hook.hk-*.command` entry is set in `~/.gitconfig`.
/// Used to short-circuit the per-repo install when a global one is already
/// in place (overridable with `--force-local`).
fn has_global_hk_hooks() -> Result<bool> {
    let output = Command::new("git")
        .args([
            "config",
            "--global",
            "--name-only",
            "--get-regexp",
            "^hook\\.hk-.*\\.command$",
        ])
        .output()?;
    // git config --get-regexp: 0 = matches, 1 = no matches, ≥2 = real error.
    match output.status.code().unwrap_or(1) {
        0 => Ok(!output.stdout.is_empty()),
        1 => Ok(false),
        code => {
            bail!(
                "git config --get-regexp failed (exit {}): {}",
                code,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }
}

fn local_hook_command(use_mise: bool) -> OsString {
    if use_mise {
        OsString::from("mise x -- hk")
    } else {
        OsString::from("hk")
    }
}

/// The directory hook commands append to `PATH` so a bare `hk` still resolves
/// when Git is launched with a minimal `PATH` (GUI Git clients, IDEs), where
/// the user's shell `PATH` is not present. It is appended, so an `hk` the
/// environment already provides keeps priority. `None` with `--mise`, where
/// mise is the launcher.
fn local_path_fallback(use_mise: bool) -> Option<PathBuf> {
    if use_mise {
        return None;
    }
    let exe = std::env::current_exe().ok()?;
    let hk = stable_hk_path(&exe, std::env::var_os("PATH"));
    hk.parent().map(Path::to_path_buf)
}

/// A path to the running hk that survives upgrades.
///
/// `current_exe()` is the resolved binary, such as
/// `~/.local/share/mise/installs/hk/2.3.1/bin/hk` or a Homebrew Cellar path.
/// That version's directory is deleted when the version is pruned or
/// upgraded, and every hook then fails with `hk: not found`. Prefer, in order:
/// the mise shim, which resolves the active version when it runs, if hk is a
/// mise install; or the `hk` on `PATH` that points at this same binary (a
/// Homebrew or package-manager symlink). Otherwise use the binary itself.
fn stable_hk_path(exe: &Path, path_var: Option<OsString>) -> PathBuf {
    if let Some(shim) = mise_shim_for(exe) {
        return shim;
    }
    let exe_real = exe.canonicalize().unwrap_or_else(|_| exe.to_path_buf());
    let name = exe.file_name().unwrap_or(OsStr::new("hk"));
    for dir in path_var.iter().flat_map(std::env::split_paths) {
        if !dir.is_absolute() {
            continue;
        }
        let candidate = dir.join(name);
        if candidate.canonicalize().is_ok_and(|real| real == exe_real) {
            return candidate;
        }
    }
    exe.to_path_buf()
}

/// The mise shim for `exe` when it is inside mise's `installs` directory.
fn mise_shim_for(exe: &Path) -> Option<PathBuf> {
    let components: Vec<_> = exe.components().collect();
    let installs = components
        .iter()
        .position(|c| c.as_os_str() == "installs")?;
    let data_dir: PathBuf = components[..installs].iter().collect();
    // `<data dir>/installs/<tool>/<version>/...`: the tool must be hk.
    if components.get(installs + 1)?.as_os_str() != "hk" {
        return None;
    }
    let shim = data_dir.join("shims").join(exe.file_name()?);
    std::fs::symlink_metadata(&shim).ok().map(|_| shim)
}

fn global_hook_command(use_mise: bool) -> Result<OsString> {
    if use_mise {
        let mise = xx::file::which("mise")
            .ok_or_else(|| eyre::eyre!("could not find mise on PATH for global hook install"))?;
        Ok(mise_hook_command(&mise))
    } else {
        let hk = stable_hk_path(&std::env::current_exe()?, std::env::var_os("PATH"));
        Ok(hk_hook_command(&hk))
    }
}

fn global_hook_events() -> Result<Vec<String>> {
    if Config::project_config_exists() {
        return Ok(hook_events(&Config::get()?));
    }
    Ok(CORE_GLOBAL_EVENTS
        .iter()
        .map(|event| event.to_string())
        .collect())
}

fn hook_events(config: &Config) -> Vec<String> {
    config
        .hooks
        .iter()
        .filter(|(name, hook)| hook.enabled && name.as_str() != "check" && name.as_str() != "fix")
        .map(|(name, _)| name.clone())
        .collect()
}

fn hk_hook_command(hk: &Path) -> OsString {
    shell_path_for_command(hk)
}

fn mise_hook_command(mise: &Path) -> OsString {
    let mut command = shell_path_for_command(mise);
    command.push(" x hk -- hk");
    command
}

fn shell_path_for_command(path: &Path) -> OsString {
    if let Some(home_relative) = home_relative_command_path(path) {
        return home_relative;
    }
    shell_quote_path(path)
}

fn home_relative_command_path(path: &Path) -> Option<OsString> {
    let home = dirs::home_dir()?;
    let relative = path.strip_prefix(home).ok()?;
    if relative.as_os_str().is_empty() || !is_shell_safe_path(relative) {
        return None;
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let mut path = b"~/".to_vec();
        path.extend_from_slice(relative.as_os_str().as_bytes());
        Some(OsString::from_vec(path))
    }

    #[cfg(not(unix))]
    {
        Some(OsString::from(format!("~/{}", relative.display())))
    }
}

fn is_shell_safe_path(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        path.as_os_str().as_bytes().iter().all(is_shell_safe_byte)
    }

    #[cfg(not(unix))]
    {
        path.to_string_lossy()
            .bytes()
            .all(|b| is_shell_safe_byte(&b))
    }
}

fn is_shell_safe_byte(b: &u8) -> bool {
    matches!(
        b,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'_'
            | b'@'
            | b'%'
            | b'+'
            | b'='
            | b':'
            | b','
            | b'.'
            | b'/'
            | b'-'
    )
}

fn shell_quote_path(path: &Path) -> OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let bytes = path.as_os_str().as_bytes();
        if bytes.iter().all(is_shell_safe_byte) {
            return OsString::from_vec(bytes.to_vec());
        }

        let mut quoted = vec![b'\''];
        for b in bytes {
            if *b == b'\'' {
                quoted.extend_from_slice(b"'\\''");
            } else {
                quoted.push(*b);
            }
        }
        quoted.push(b'\'');
        OsString::from_vec(quoted)
    }

    #[cfg(not(unix))]
    {
        let path = path.to_string_lossy();
        if path.bytes().all(|b| is_shell_safe_byte(&b)) {
            OsString::from(path.as_ref())
        } else {
            OsString::from(format!("'{}'", path.replace('\'', r#"'\''"#)))
        }
    }
}
/// Git aggregates `hook.<name>.command` values across scopes, so a local
/// install on top of a global one fires hk twice per event. Warn the user
/// and point them at the `enabled = false` escape hatch.
fn warn_if_global_overlap(events: &[String]) {
    let mut overlapping: Vec<&str> = Vec::new();
    for event in events {
        let key = format!("hook.hk-{event}.command");
        if let Ok(output) = Command::new("git")
            .args(["config", "--global", "--get", key.as_str()])
            .output()
            && output.status.success()
            && !output.stdout.is_empty()
        {
            overlapping.push(event);
        }
    }
    if overlapping.is_empty() {
        return;
    }
    warn!(
        "both global (~/.gitconfig) and local hk hooks are active for: {}. Git will run hk twice per event. To run only the local install, disable the global entries in this repo: {}",
        overlapping.join(", "),
        overlapping
            .iter()
            .map(|e| format!("`git config --local hook.hk-{e}.enabled false`"))
            .collect::<Vec<_>>()
            .join(" ; ")
    );
}

fn install_global(events: &[String], command: &OsStr) -> Result<()> {
    remove_config_entries("--global")?;
    for event in events {
        write_config_hook("--global", command, event, true)?;
    }
    info!(
        "Installed hk global hooks in ~/.gitconfig for: {}",
        events.join(", ")
    );
    info!(
        "In repos without an hk.pkl, hk exits silently — add one with `hk init` to enable hooks."
    );
    Ok(())
}

fn install_local_config(
    events: &[String],
    command: &OsStr,
    path_fallback: Option<&Path>,
) -> Result<()> {
    let command = with_path_fallback(command, path_fallback);
    for event in events {
        write_config_hook("--local", &command, event, false)?;
        info!("Installed hk hook via git config: hook.hk-{event}.command");
    }
    Ok(())
}

fn install_local_shims(
    events: &[String],
    command: &OsStr,
    path_fallback: Option<&Path>,
) -> Result<()> {
    let git_path = git_util::find_git_path()?;
    let hooks = match git_util::worktree_hooks_path() {
        Some(path) => {
            xx::file::mkdirp(&path)?;
            path
        }
        None => {
            check_hooks_path_config()?;
            git_util::resolve_git_hooks_dir(&git_path)?
        }
    };
    for event in events {
        let hook_file = hooks.join(event);
        xx::file::write(
            &hook_file,
            git_hook_content(&command.to_string_lossy(), event, path_fallback),
        )?;
        xx::file::make_executable(&hook_file)?;
        info!("Installed hk hook: {}", hook_file.display());
    }
    Ok(())
}

/// Write both `hook.hk-<event>.command` and `hook.hk-<event>.event` at the
/// given scope (`--local` or `--global`).
fn write_config_hook(
    scope: &str,
    command: &OsStr,
    event: &str,
    staged_pre_commit: bool,
) -> Result<()> {
    let name = format!("hk-{event}");
    let cmd_key = format!("hook.{name}.command");
    let event_key = format!("hook.{name}.event");
    // Mirror the shim's HK=0 escape hatch so users can still disable hooks
    // with `HK=0 git commit` under config-based hooks.
    let mut cmd_value = OsString::from(r#"test "${HK:-1}" = "0" || "#);
    cmd_value.push(command);
    cmd_value.push(hook_run_args(event, staged_pre_commit));

    run_git([
        OsString::from("config"),
        OsString::from(scope),
        OsString::from(cmd_key.as_str()),
        cmd_value,
    ])?;
    // .event is multi-valued; replace-all keeps re-install idempotent.
    run_git([
        OsString::from("config"),
        OsString::from(scope),
        OsString::from("--replace-all"),
        OsString::from(event_key.as_str()),
        OsString::from(event),
    ])?;
    Ok(())
}

fn remove_local_config_entries() -> Result<usize> {
    remove_config_entries("--local")
}

pub(crate) fn lock_local_config() -> Result<File> {
    // --git-path resolves the shared config for linked worktrees and also
    // honors GIT_DIR, unlike constructing a path from the working tree.
    let output = Command::new("git")
        .args(["rev-parse", "--git-path", "config"])
        .output()?;
    if !output.status.success() {
        bail!(
            "git rev-parse --git-path config failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    #[cfg(unix)]
    let config = {
        use std::os::unix::ffi::OsStringExt;
        let mut path = output.stdout;
        while matches!(path.last(), Some(b'\n' | b'\r')) {
            path.pop();
        }
        OsString::from_vec(path)
    };
    #[cfg(not(unix))]
    let config = OsString::from(String::from_utf8(output.stdout)?.trim_end());
    #[cfg(unix)]
    let config_mode = {
        use std::os::unix::fs::PermissionsExt;
        (std::fs::metadata(Path::new(&config))?.permissions().mode() & 0o666) | 0o200
    };
    let mut lock_path = config;
    lock_path.push(".hk-install.lock");
    let lock_path = Path::new(&lock_path);
    let lock = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(lock_path)
    {
        Ok(lock) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                // Preserve the config's shared permissions despite the
                // process umask. Keep owner write access even if the config
                // itself is read-only: Git can replace such a config, and
                // subsequent installs still need to open this lock.
                lock.set_permissions(std::fs::Permissions::from_mode(config_mode))?;
            }
            lock
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another process may see a new file before its creator has
            // applied shared-repository permissions.
            let mut attempts = 0;
            loop {
                match OpenOptions::new().write(true).open(lock_path) {
                    Ok(lock) => break lock,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::PermissionDenied
                            && attempts < 10 =>
                    {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
        Err(error) => return Err(error.into()),
    };
    lock.lock()?;
    Ok(lock)
}

pub(crate) fn remove_config_entries(scope: &str) -> Result<usize> {
    let output = Command::new("git")
        .args([
            "config",
            scope,
            "--name-only",
            "--get-regexp",
            "^hook\\.hk-",
        ])
        .output()?;
    // git config --get-regexp: 0 = matches, 1 = no matches, ≥2 = real error
    // (e.g. unreadable config). Don't conflate "nothing to remove" with a
    // failed uninstall.
    let code = output.status.code().unwrap_or(1);
    if code == 1 {
        return Ok(0);
    }
    if !output.status.success() {
        bail!(
            "git config --get-regexp failed (exit {}): {}",
            code,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let keys: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    // Dedupe since multi-valued keys appear once per value.
    let mut seen = std::collections::BTreeSet::new();
    let mut removed = 0;
    for key in keys {
        if seen.insert(key.clone()) {
            run_git(["config", scope, "--unset-all", key.as_str()])?;
            // Count one per hook event, not one per key (command + event).
            if key.ends_with(".command") {
                removed += 1;
            }
        }
    }
    Ok(removed)
}

pub(crate) fn remove_local_shims() -> Result<usize> {
    let git_path = match git_util::find_git_path() {
        Ok(p) => p,
        Err(_) => return Ok(0),
    };
    let hooks = match git_util::worktree_hooks_path() {
        Some(path) => path,
        None => git_util::resolve_git_hooks_dir(&git_path)?,
    };
    if !hooks.is_dir() {
        return Ok(0);
    }
    let mut removed = 0;
    for p in xx::file::ls(&hooks)? {
        let content = match xx::file::read_to_string(&p) {
            Ok(content) => content,
            Err(_) => continue,
        };
        // Match the HK=0 guard that every hk-written shim has. This is more
        // specific than `hk run` alone, which could appear in an unrelated
        // user-written hook.
        if content.contains(r#"test "${HK:-1}" = "0""#) && content.contains("hk run") {
            xx::file::remove_file(&p)?;
            info!("removed hook: {}", xx::file::display_path(&p));
            removed += 1;
        }
    }
    Ok(removed)
}

fn run_git<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    let status = Command::new("git").args(&args).status()?;
    if !status.success() {
        let args = args
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        bail!("git {args} failed");
    }
    Ok(())
}

fn git_hook_content(hk: &str, hook: &str, path_fallback: Option<&Path>) -> String {
    match path_fallback {
        // The braces keep the `HK=0` escape hatch on one line.
        Some(dir) => format!(
            r#"#!/bin/sh
test "${{HK:-1}}" = "0" || {{ PATH="$PATH":{}; export PATH; exec {hk} run {hook} --from-hook "$@"; }}
"#,
            shell_quote_path(dir).to_string_lossy()
        ),
        None => format!(
            r#"#!/bin/sh
test "${{HK:-1}}" = "0" || exec {hk} run {hook} --from-hook "$@"
"#,
        ),
    }
}

/// `command` run with `dir` appended to `PATH`, for a hook command string
/// that Git runs through the shell.
fn with_path_fallback(command: &OsStr, dir: Option<&Path>) -> OsString {
    let Some(dir) = dir else {
        return command.to_os_string();
    };
    let mut with = OsString::from(r#"PATH="$PATH":"#);
    with.push(shell_quote_path(dir));
    with.push(" ");
    with.push(command);
    with
}

fn hook_run_args(event: &str, staged_pre_commit: bool) -> OsString {
    let staged = if staged_pre_commit && event == "pre-commit" {
        " --staged"
    } else {
        ""
    };
    // Config-based hooks receive their hook arguments from Git automatically.
    // Forwarding "$@" here would pass every argument twice.
    OsString::from(format!(r#" run {event} --from-hook{staged}"#))
}

fn check_hooks_path_config() -> Result<()> {
    let check_config = |scope: &str| -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["config", scope, "--get", "core.hooksPath"])
            .output()?;

        if output.status.success() {
            let value = String::from_utf8(output.stdout)?.trim().to_string();
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
        Ok(None)
    };

    let mut warnings = Vec::new();

    if let Ok(Some(path)) = check_config("--global") {
        warnings.push(format!(
            "core.hooksPath is set globally to '{}'. This may prevent hk hooks from running.",
            path
        ));
        warnings
            .push("Run 'git config --global --unset-all core.hooksPath' to remove it.".to_string());
    }

    if let Ok(Some(path)) = check_config("--local") {
        warnings.push(format!(
            "core.hooksPath is set locally to '{}'. This may prevent hk hooks from running.",
            path
        ));
        warnings
            .push("Run 'git config --local --unset-all core.hooksPath' to remove it.".to_string());
    }

    for warning in &warnings {
        warn!("{}", warning);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn global_hk_command_uses_quoted_absolute_path() {
        assert_eq!(
            hk_hook_command(Path::new("/tmp/hk bin/hk")).to_string_lossy(),
            "'/tmp/hk bin/hk'"
        );
    }

    #[test]
    fn global_mise_command_requests_hk_tool_explicitly() {
        assert_eq!(
            mise_hook_command(Path::new("/opt/homebrew/bin/mise")).to_string_lossy(),
            "/opt/homebrew/bin/mise x hk -- hk"
        );
    }

    #[test]
    fn global_hook_command_uses_tilde_for_home_relative_paths() {
        let home = dirs::home_dir().expect("home directory should exist");

        assert_eq!(
            hk_hook_command(&home.join(".local/bin/hk")).to_string_lossy(),
            "~/.local/bin/hk"
        );
    }

    #[test]
    fn stable_hk_path_prefers_the_mise_shim_over_a_versioned_install() {
        let data = tempfile::tempdir().unwrap();
        let exe = data.path().join("installs/hk/2.3.1/bin/hk");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, "").unwrap();
        std::fs::create_dir_all(data.path().join("shims")).unwrap();
        std::fs::write(data.path().join("shims/hk"), "").unwrap();
        // A PATH entry for the versioned directory, as `mise activate` adds.
        let path = std::env::join_paths([exe.parent().unwrap()]).unwrap();
        assert_eq!(
            stable_hk_path(&exe, Some(path)),
            data.path().join("shims/hk")
        );
    }

    #[test]
    fn stable_hk_path_ignores_other_mise_tools_and_missing_shims() {
        let data = tempfile::tempdir().unwrap();
        let other = data.path().join("installs/other/1/bin/hk");
        let exe = data.path().join("installs/hk/2.3.1/bin/hk");
        for p in [&other, &exe] {
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "").unwrap();
        }
        // No shim exists yet.
        assert_eq!(stable_hk_path(&exe, None), exe);
        std::fs::create_dir_all(data.path().join("shims")).unwrap();
        std::fs::write(data.path().join("shims/hk"), "").unwrap();
        assert_eq!(stable_hk_path(&other, None), other);
    }

    #[cfg(unix)]
    #[test]
    fn stable_hk_path_uses_a_path_symlink_to_the_binary() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Cellar/hk/2.3.1/bin/hk");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, "").unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(&exe, bin.join("hk")).unwrap();
        // A different hk earlier on PATH is not this binary and is skipped.
        let elsewhere = dir.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(elsewhere.join("hk"), "other").unwrap();
        let path = std::env::join_paths([elsewhere, bin.clone()]).unwrap();
        assert_eq!(stable_hk_path(&exe, Some(path)), bin.join("hk"));
        assert_eq!(stable_hk_path(&exe, None), exe);
    }

    #[test]
    fn shim_and_config_command_append_the_fallback_dir_to_path() {
        let dir = Path::new("/opt/hk bin");
        let shim = git_hook_content("hk", "pre-commit", Some(dir));
        assert!(shim.contains(r#"test "${HK:-1}" = "0" || { PATH="$PATH":'/opt/hk bin'; export PATH; exec hk run pre-commit --from-hook "$@"; }"#), "{shim}");
        assert!(is_hk_shim_like(&shim));
        let plain = git_hook_content("hk", "pre-commit", None);
        assert!(plain.contains(r#"|| exec hk run pre-commit --from-hook "$@""#));
        assert_eq!(
            with_path_fallback(OsStr::new("hk"), Some(dir)),
            OsString::from(r#"PATH="$PATH":'/opt/hk bin' hk"#)
        );
        assert_eq!(
            with_path_fallback(OsStr::new("mise x -- hk"), None),
            OsString::from("mise x -- hk")
        );
    }

    fn is_hk_shim_like(content: &str) -> bool {
        content.contains(r#"test "${HK:-1}" = "0""#) && content.contains("hk run")
    }

    #[test]
    fn local_mise_command_keeps_existing_behavior() {
        assert_eq!(local_hook_command(true), OsString::from("mise x -- hk"));
        assert_eq!(local_hook_command(false), OsString::from("hk"));
    }

    #[test]
    fn hook_run_args_adds_staged_only_for_pre_commit() {
        assert_eq!(
            hook_run_args("pre-commit", true),
            OsString::from(r#" run pre-commit --from-hook --staged"#)
        );
        assert_eq!(
            hook_run_args("pre-push", true),
            OsString::from(r#" run pre-push --from-hook"#)
        );
        assert_eq!(
            hook_run_args("pre-commit", false),
            OsString::from(r#" run pre-commit --from-hook"#)
        );
    }

    #[test]
    fn disabled_hooks_are_not_installable_events() {
        let mut config = Config::default();
        config.hooks.insert(
            "pre-commit".to_string(),
            crate::hook::Hook {
                enabled: false,
                ..Default::default()
            },
        );
        config
            .hooks
            .insert("pre-push".to_string(), crate::hook::Hook::default());

        assert_eq!(hook_events(&config), vec!["pre-push"]);
    }

    #[cfg(unix)]
    #[test]
    fn shell_quote_path_preserves_non_utf8_bytes() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let path = Path::new(OsStr::from_bytes(b"/tmp/hk-\xFF/bin/hk"));
        let quoted = shell_quote_path(path);

        assert!(quoted.into_vec().contains(&0xFF));
    }
}
