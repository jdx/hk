use crate::{Result, config::Config, git::Git, settings::Settings, tera::Context};
use std::path::PathBuf;

#[derive(usage_rs::Args)]
pub(crate) struct HookOptions {
    /// Run on specific files
    #[usage(
        arg,
        value_hint = ValueHint::FilePath,
        conflicts("--all", "--files0-from", "--pr", "--staged", "--unstaged")
    )]
    pub files: Option<Vec<PathBuf>>,
    /// Run on all files instead of just staged files
    #[usage(short, long, conflicts("--staged", "--unstaged"))]
    pub all: bool,
    /// Run check command instead of fix command
    #[usage(short, long, overrides = "--fix")]
    pub check: bool,
    /// Exclude files that otherwise would have been selected
    #[usage(short, long, value_hint = ValueHint::FilePath)]
    pub exclude: Option<Vec<String>>,
    /// Run fix command instead of check command
    /// (this is the default behavior unless HK_FIX=0)
    #[usage(short, long, overrides = "--check")]
    pub fix: bool,
    /// Run on files that match these glob patterns
    #[usage(short, long, value_hint = ValueHint::FilePath)]
    pub glob: Option<Vec<String>>,
    /// Output the plan as JSON when combined with --plan or --why
    #[usage(short = 'J', long)]
    pub json: bool,
    /// Print the plan instead of running the hook
    #[usage(short = 'P', long)]
    pub plan: bool,
    /// Run only specific step(s)
    #[usage(short = 'S', long)]
    pub step: Vec<String>,
    /// Show detailed reasons for inclusion/exclusion. Pass a step name to focus on one step, or omit the value to show reasons for all steps. Implies --plan.
    #[usage(short = 'W', long, value_name = "STEP", default_missing = "")]
    pub why: Option<String>,
    /// Abort on first failure
    #[usage(long, overrides = "--no-fail-fast")]
    pub fail_fast: bool,
    /// Read the exact file list from a NUL-delimited file, or from stdin with `-` (except hooks that reserve stdin)
    #[usage(
        long,
        value_name = "PATH",
        value_hint = ValueHint::FilePath,
        conflicts("--all", "--from-ref", "--glob", "--pr", "--staged", "--to-ref", "--unstaged")
    )]
    pub files0_from: Option<PathBuf>,
    /// Select human or machine-readable execution output
    #[usage(long, value_enum)]
    pub format: Option<crate::structured_output::OutputFormat>,
    /// Invoked by an installed git hook — gracefully exit 0 when no hk.pkl is
    /// present or the event isn't defined. Set automatically by `hk install`.
    #[usage(long, hide)]
    pub from_hook: bool,
    /// Start reference for checking files (requires --to-ref)
    #[usage(long)]
    pub from_ref: Option<String>,
    /// Continue on failures (opposite of --fail-fast)
    #[usage(long, overrides = "--fail-fast")]
    pub no_fail_fast: bool,
    /// Disable auto-staging of fixed files
    #[usage(long, overrides = "--stage")]
    pub no_stage: bool,
    /// Check only files changed in the current PR/branch (shortcut for --from-ref DEFAULT_BRANCH --to-ref HEAD)
    #[usage(long, conflicts("--all", "--from-ref", "--glob", "--to-ref"))]
    pub pr: bool,
    /// Reject commands with unknown or destructive effects before execution
    #[usage(long)]
    pub safe: bool,
    /// Write normalized diagnostics as SARIF
    #[usage(long, value_name = "PATH", value_hint = ValueHint::FilePath)]
    pub sarif: Option<PathBuf>,
    /// Skip specific step(s)
    #[usage(long, value_name = "STEP")]
    pub skip_step: Vec<String>,
    /// Enable auto-staging of fixed files
    #[usage(long, overrides = "--no-stage")]
    pub stage: bool,
    /// Run on staged files only without stashing unstaged changes
    #[usage(
        long,
        conflicts(
            "--all",
            "--from-ref",
            "--glob",
            "--pr",
            "--stash",
            "--to-ref",
            "--unstaged"
        )
    )]
    pub staged: bool,
    /// Stash method to use for git hooks
    #[usage(long, choices("git", "patch-file", "none"))]
    pub stash: Option<String>,
    /// Display statistics about files matching each step
    #[usage(long)]
    pub stats: bool,
    /// End reference for checking files (requires --from-ref)
    #[usage(long)]
    pub to_ref: Option<String>,
    /// Run on unstaged and untracked files only (excludes staged files),
    /// without stashing. Useful for linting files an agent just changed.
    #[usage(
        long,
        conflicts(
            "--all",
            "--from-ref",
            "--glob",
            "--pr",
            "--stash",
            "--to-ref",
            "--staged"
        )
    )]
    pub unstaged: bool,
    /// Prefilled tera context
    #[usage(skip)]
    pub tctx: Context,
}

impl HookOptions {
    fn validate(&self) -> Result<()> {
        if self.staged && self.stash.is_some() {
            return Err(eyre::eyre!(
                "argument '--staged' cannot be used with '--stash <STASH>'"
            ));
        }
        if self.unstaged && self.stash.is_some() {
            return Err(eyre::eyre!(
                "argument '--unstaged' cannot be used with '--stash <STASH>'"
            ));
        }
        Ok(())
    }

    pub(crate) fn reads_file_list_from_stdin(&self) -> bool {
        self.files0_from.as_deref() == Some(std::path::Path::new("-"))
    }

    fn load_files0(&mut self) -> Result<()> {
        use std::io::Read;

        let Some(source) = self.files0_from.take() else {
            return Ok(());
        };
        let mut bytes = Vec::new();
        if source == std::path::Path::new("-") {
            std::io::stdin().read_to_end(&mut bytes)?;
        } else {
            std::fs::File::open(&source)?.read_to_end(&mut bytes)?;
        }

        let mut files = Vec::new();
        for raw in bytes.split(|byte| *byte == 0) {
            if raw.is_empty() {
                continue;
            }
            #[cfg(unix)]
            let path = {
                use std::os::unix::ffi::OsStringExt;
                std::str::from_utf8(raw).map_err(|_| {
                    eyre::eyre!("--files0-from contains a path that is not valid UTF-8")
                })?;
                PathBuf::from(std::ffi::OsString::from_vec(raw.to_vec()))
            };
            #[cfg(not(unix))]
            let path = PathBuf::from(String::from_utf8(raw.to_vec()).map_err(|_| {
                eyre::eyre!("--files0-from contains a path that is not valid UTF-8")
            })?);
            files.push(path);
        }
        self.files = Some(files);
        Ok(())
    }

    pub fn should_stage(&self) -> Option<bool> {
        if self.stage {
            Some(true)
        } else if self.no_stage {
            Some(false)
        } else {
            None
        }
    }

    pub(crate) async fn run(mut self, name: &str) -> Result<()> {
        self.validate()?;
        self.load_files0()?;
        // Under `--from-hook`, short-circuit *before* loading the config. A
        // broken user-global hkrc (or missing `pkl`) shouldn't fail every
        // `git commit` in a repo that doesn't even use hk — which is the
        // main risk under `hk install --global`.
        if self.from_hook && !Config::project_config_exists() {
            log::debug!("no hk config found for {name}, skipping (--from-hook)");
            crate::structured_output::emit_noop_run(
                Settings::cli_output_format(),
                name,
                chrono::Utc::now().to_rfc3339(),
                0,
                vec![],
                "no project configuration found for installed hook",
                self.sarif.as_deref(),
            )?;
            return Ok(());
        }
        let config = Config::get()?;
        if self.pr {
            let repo = Git::new()?;
            let default_branch = config
                .default_branch
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| repo.default_branch().unwrap_or_else(|_| "main".to_string()));
            self.from_ref = Some(default_branch);
            self.to_ref = Some("HEAD".to_string());
        }
        // Validate --json. Skip when the user passed --trace (or has
        // HK_TRACE/HK_JSON set) — in that case the global --json flag
        // controls trace output and legitimately populates this field too.
        if self.json
            && !self.plan
            && self.why.is_none()
            && !Settings::cli_trace()
            && !*crate::env::HK_JSON
            && !matches!(*crate::env::HK_TRACE, crate::env::TraceMode::Json)
        {
            return Err(eyre::eyre!("--json requires --plan or --why"));
        }
        match config.hooks.get(name) {
            Some(hook) => {
                if !hook.enabled {
                    log::debug!("hook '{name}' is disabled, skipping");
                    crate::structured_output::emit_noop_run(
                        Settings::cli_output_format(),
                        name,
                        chrono::Utc::now().to_rfc3339(),
                        0,
                        vec![],
                        "hook disabled by configuration",
                        self.sarif.as_deref(),
                    )?;
                    return Ok(());
                }
                if self.stats {
                    hook.stats(self, name).await?;
                } else if self.plan || self.why.is_some() {
                    hook.plan(self).await?;
                } else {
                    hook.run(self).await?;
                }
                Ok(())
            }
            None => {
                if self.from_hook {
                    log::debug!(
                        "hook '{name}' not defined in {}, skipping (--from-hook)",
                        config.path.display()
                    );
                    crate::structured_output::emit_noop_run(
                        Settings::cli_output_format(),
                        name,
                        chrono::Utc::now().to_rfc3339(),
                        0,
                        vec![],
                        "hook not defined in project configuration",
                        self.sarif.as_deref(),
                    )?;
                    return Ok(());
                }
                let hook_names: Vec<&str> = config.hooks.keys().map(|s| s.as_str()).collect();
                let msg = if let Some(suggestion) = xx::suggest::did_you_mean(name, &hook_names) {
                    format!("Hook '{}' not found. {}", name, suggestion)
                } else {
                    format!("Hook '{}' not found", name)
                };
                Err(eyre::eyre!("{}", msg))
            }
        }
    }
}
