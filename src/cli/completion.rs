use crate::{Result, cli::Cli};
use usage_rs::install::{self, OnForeign, Wrote};

/// Generates shell completion scripts
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "read")]
pub struct Completion {
    /// The shell to generate completion for
    #[usage(arg)]
    shell: String,

    /// Install the script where this shell looks for it, instead of printing it
    ///
    /// Writes the script file and nothing else: no shell rc file and no PowerShell profile is
    /// edited. Where a shell needs a one-time line of its own — zsh's `fpath+=`, PowerShell's
    /// dot-source — it is printed for you to add.
    #[usage(long, effect = "write")]
    install: bool,

    /// Replace a file at the target path that hk did not write
    #[usage(long, requires = "--install", effect = "write")]
    force: bool,
}

impl Completion {
    pub async fn run(&self) -> Result<()> {
        let shell = usage_rs::complete::Shell::from_name(&self.shell)
            .ok_or_else(|| eyre::eyre!("unsupported shell: {}", self.shell))?;
        if !self.install {
            print!("{}", Cli::completion_script(shell));
            return Ok(());
        }
        self.install(shell)
    }

    /// Put the script where this shell looks for it, and say what is left to do.
    ///
    /// The location comes from usage rather than from a table here: the same resolver answers for
    /// every CLI built on it, so `hk completion zsh --install` and `usage g completion zsh hk
    /// --install` cannot disagree about where an hk completion lives.
    fn install(&self, shell: usage_rs::complete::Shell) -> Result<()> {
        let on_foreign = if self.force {
            OnForeign::Overwrite
        } else {
            OnForeign::Refuse
        };
        // The environment is described from this process rather than reached for inside the
        // resolver, which is what lets a test point the same code path somewhere harmless.
        let done = Cli::install_completion(shell, &install::Env::from_process(), on_foreign)
            .map_err(|err| match &err {
                install::Error::Foreign { .. } => eyre::eyre!(
                    "{err}\n\nPass --force to replace it, or redirect the script yourself."
                ),
                _ => eyre::Report::new(err),
            })?;

        // All of this goes to stderr, so stdout stays empty under `--install`: nothing was
        // generated for the caller to capture, and a note about a write is not the thing written.
        eprintln!("installing to {}", done.plan.path.display());
        if done.wrote == Wrote::Unchanged {
            eprintln!("already up to date");
        }
        if let Some(line) = done.plan.loading.instruction() {
            let file = match &done.plan.loading {
                install::Loading::Manual { file, .. } => file.as_str(),
                _ => "your shell's startup file",
            };
            eprintln!("\nadd this to {file}, once:\n\n{line}\n");
        }
        if let Some(note) = done.plan.note {
            eprintln!("note: {note}");
        }
        Ok(())
    }
}
