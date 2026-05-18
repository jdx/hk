use crate::{Result, config::Config, env, git_util};
use eyre::bail;
use log::warn;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

/// Hook events installed by default for `hk install --global` when no project
/// config is available to enumerate a more specific set.
const CORE_GLOBAL_EVENTS: &[&str] = &["commit-msg", "pre-commit", "pre-push", "prepare-commit-msg"];

/// Sets up git hooks to run hk.
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
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "i")]
pub struct Install {
    /// Install local hooks even when hk is already configured globally
    /// (any `hook.hk-*` entry in `~/.gitconfig`). By default a per-repo
    /// install is skipped in that case to avoid hk firing twice per
    /// event. Not compatible with `--global`.
    #[clap(long, verbatim_doc_comment, conflicts_with = "global")]
    force_local: bool,

    /// Recommended. Install at user level (~/.gitconfig) so every repo
    /// on this machine gets hk hooks. Requires Git 2.54 or newer. In
    /// repos without an `hk.pkl`, the installed hook is a silent no-op.
    #[clap(long, verbatim_doc_comment)]
    global: bool,

    /// Force using the legacy `.git/hooks/` script shims instead of Git
    /// 2.54+ config-based hooks. Not compatible with `--global`.
    #[clap(long, verbatim_doc_comment, conflicts_with = "global")]
    legacy: bool,

    /// Use `mise x` to execute hooks. With this, it won't
    /// be necessary to activate mise in order to run hooks
    /// with mise tools.
    ///
    /// Set HK_MISE=1 to make this default behavior.
    #[clap(long, verbatim_doc_comment)]
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
            // The global install is the single source of truth; clean up any
            // stale local install so it doesn't double-fire alongside global.
            let removed = remove_local_shims()? + remove_local_config_entries()?;
            if removed > 0 {
                println!(
                    "hk hooks already configured globally (~/.gitconfig); removed {removed} stale local hook(s) and did not install new ones. Pass `--force-local` to install per-repo hooks anyway."
                );
            } else {
                println!(
                    "hk hooks already configured globally (~/.gitconfig); skipping local install. Pass `--force-local` to install per-repo hooks anyway."
                );
            }
            return Ok(());
        }

        let command = local_hook_command(use_mise);
        let use_config_hooks = !self.legacy && git_util::git_at_least(2, 54);

        // Load and validate the project config before touching anything, so a
        // broken `hk.pkl` doesn't leave the repo with its prior hooks removed.
        let config = Config::get()?;
        let events = hook_events(&config);

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
            let result = install_local_config(&events, &command);
            warn_if_global_overlap(&events);
            result
        } else {
            install_local_shims(&events, &command)
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
        code => bail!(
            "git config --get-regexp failed (exit {}): {}",
            code,
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}

fn local_hook_command(use_mise: bool) -> OsString {
    if use_mise {
        OsString::from("mise x -- hk")
    } else {
        OsString::from("hk")
    }
}

fn global_hook_command(use_mise: bool) -> Result<OsString> {
    if use_mise {
        let mise = xx::file::which("mise")
            .ok_or_else(|| eyre::eyre!("could not find mise on PATH for global hook install"))?;
        Ok(mise_hook_command(&mise))
    } else {
        let hk = std::env::current_exe()?;
        Ok(hk_hook_command(&hk))
    }
}

fn global_hook_events() -> Result<Vec<String>> {
    if Config::project_config_exists() {
        let events = hook_events(&Config::get()?);
        if !events.is_empty() {
            return Ok(events);
        }
    }
    Ok(CORE_GLOBAL_EVENTS
        .iter()
        .map(|event| event.to_string())
        .collect())
}

fn hook_events(config: &Config) -> Vec<String> {
    config
        .hooks
        .keys()
        .filter(|h| h.as_str() != "check" && h.as_str() != "fix")
        .cloned()
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
        write_config_hook("--global", command, event)?;
    }
    println!(
        "Installed hk global hooks in ~/.gitconfig for: {}",
        events.join(", ")
    );
    println!(
        "In repos without an hk.pkl, hk exits silently — add one with `hk init` to enable hooks."
    );
    Ok(())
}

fn install_local_config(events: &[String], command: &OsStr) -> Result<()> {
    for event in events {
        write_config_hook("--local", command, event)?;
        println!("Installed hk hook via git config: hook.hk-{event}.command");
    }
    Ok(())
}

fn install_local_shims(events: &[String], command: &OsStr) -> Result<()> {
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
            git_hook_content(&command.to_string_lossy(), event),
        )?;
        xx::file::make_executable(&hook_file)?;
        println!("Installed hk hook: {}", hook_file.display());
    }
    Ok(())
}

/// Write both `hook.hk-<event>.command` and `hook.hk-<event>.event` at the
/// given scope (`--local` or `--global`).
fn write_config_hook(scope: &str, command: &OsStr, event: &str) -> Result<()> {
    let name = format!("hk-{event}");
    let cmd_key = format!("hook.{name}.command");
    let event_key = format!("hook.{name}.event");
    // Mirror the shim's HK=0 escape hatch so users can still disable hooks
    // with `HK=0 git commit` under config-based hooks.
    let mut cmd_value = OsString::from(r#"test "${HK:-1}" = "0" || "#);
    cmd_value.push(command);
    cmd_value.push(format!(r#" run {event} --from-hook "$@""#));

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

fn git_hook_content(hk: &str, hook: &str) -> String {
    format!(
        r#"#!/bin/sh
test "${{HK:-1}}" = "0" || exec {hk} run {hook} --from-hook "$@"
"#
    )
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
    fn local_mise_command_keeps_existing_behavior() {
        assert_eq!(local_hook_command(true), OsString::from("mise x -- hk"));
        assert_eq!(local_hook_command(false), OsString::from("hk"));
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
