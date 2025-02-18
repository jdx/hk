use crate::{config::Config, env, step::RunType};
use crate::{git::Git, Result};

/// Sets up git hooks to run hk
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "co")]
pub struct PreCommit {
    /// Run on all files instead of just staged files
    #[clap(short, long)]
    all: bool,
    /// Run fix command instead of run command
    /// This is the default behavior unless HK_FIX=0
    #[clap(short, long, overrides_with = "check")]
    fix: bool,
    /// Run run command instead of fix command
    #[clap(short, long, overrides_with = "fix")]
    check: bool,
    /// Force stashing even if it's disabled via HK_STASH
    #[clap(long)]
    stash: bool,
}

impl PreCommit {
    pub async fn run(&self) -> Result<()> {
        let config = Config::get()?;
        if env::HK_SKIP_HOOK.contains("pre-commit") {
            warn!("pre-commit: skipping hook due to HK_SKIP_HOOK");
            return Ok(());
        }
        let mut repo = Git::new()?;
        let run_type = if self.all {
            if !self.check && (self.fix || *env::HK_FIX) {
                RunType::FixAll
            } else {
                RunType::CheckAll
            }
        } else if !self.check && (self.fix || *env::HK_FIX) {
            RunType::Fix
        } else {
            RunType::Check
        };
        if !self.all {
            repo.stash_unstaged(self.stash)?;
        }
        let mut result = if let Some(hook) = &config.pre_commit {
            config.run_hook(hook, run_type, &repo).await
        } else {
            Ok(())
        };

        if let Err(err) = repo.pop_stash() {
            if result.is_ok() {
                result = Err(err);
            } else {
                warn!("Failed to pop stash: {}", err);
            }
        }
        result
    }
}

// async fn run_step<P: AsRef<Path>>(step: &Step, matches: &[P], ctx: &StepContext) -> Result<()> {
//     let matches = matches
//         .iter()
//         .map(|m| m.as_ref().to_path_buf())
//         .collect_vec();
//     let _lock_all = ctx.lock_all.read().await;
//     let mut locks = IndexMap::new();
//     let mut locks_read = IndexMap::new();
//     for p in &matches {
//         let file_lock = get_file_lock(p).await;
//         locks.insert(p, file_lock.clone());
//     }
//     for (p, lock) in &locks {
//         locks_read.insert(p.to_path_buf(), lock.read().await);
//     }
//     let mpr = ensembler::MultiProgressReport::get();
//     let pr = Arc::new(mpr.add(&step.name));
//     // if let Some(run) = &step.list_files_with_errors {
//     //     let mut ctx = tera::Context::default();
//     //     let matches_ref: Vec<&Path> = matches.iter().map(|p| p.as_ref()).collect();
//     //     ctx.with_staged_files(&matches_ref);
//     //     let run = tera::render(run, &ctx)?;
//     //     info!("running {}", run);
//     //     let out = ensembler::CmdLineRunner::new("sh")
//     //         .arg("-c")
//     //         .arg(run)
//     //         .with_pr(pr.clone())
//     //         .execute()?;
//     //     let files_with_errors = out
//     //         .stdout
//     //         .split('\n')
//     //         .map(|s| PathBuf::from(s.trim()))
//     //         .filter(|p| p.exists())
//     //         .collect_vec();
//     //     if !files_with_errors.is_empty() {
//     //         pr.set_message(format!(
//     //             "Fixing {} files with errors",
//     //             files_with_errors.len()
//     //         ));
//     //         let mut locks = IndexMap::new();
//     //         for p in &files_with_errors {
//     //             let file_lock = get_file_lock(p).await;
//     //             locks.insert(p, file_lock.clone());
//     //         }
//     //         let mut ctx = tera::Context::default();
//     //         ctx.with_files(&files_with_errors);
//     //         let fix = tera::render(step.fix.as_deref().unwrap(), &ctx)?;
//     //         info!("fixing {}", fix);
//     //         ensembler::CmdLineRunner::new("sh")
//     //             .arg("-c")
//     //             .arg(fix)
//     //             .with_pr(pr.clone())
//     //             .execute()?;
//     //         // TODO: re-use existing repo for perf
//     //         let mut repo = Git::new()?;
//     //         let paths: Vec<String> = files_with_errors
//     //             .iter()
//     //             .map(|p| p.to_str().unwrap().to_string())
//     //             .collect();
//     //         repo.add(&paths.iter().map(|s| s.as_str()).collect_vec())?;
//     //     }
//     // } else if let Some(run) = step.render_error_json.clone() {
//     // } else if let Some(plugin) = &step.plugin {
//     // if let Some(plugin) = &step.plugin {
//     //     if let Some(plugin) = CORE_PLUGINS.get(plugin.as_str()) {
//     //         let mut diagnostics = Vec::new();
//     //         let mut actions = Vec::new();
//     //         let mut times = 3;
//     //         loop {
//     //             times -= 1;
//     //             (diagnostics, actions) = plugin.lint(&matches)?;
//     //             if actions.is_empty() || times == 0 {
//     //                 break
//     //             } else {
//     //                 apply_actions(&actions)?;
//     //             }
//     //         }
//     //         if !diagnostics.is_empty() {
//     //             let msg = diagnostics.into_iter().map(|d| d.to_string()).collect_vec().join("\n");
//     //             return Err(Error::Diagnostic(msg));
//     //         }
//     //     } else {
//     //         warn!("Plugin {} not found", plugin);
//     //     }
//     // }
//     Ok(())
// }

// // fn apply_actions(actions: &[CodeAction]) -> Result<()> {
// //     let mut repo = Git::new()?;
// //     for action in actions {
// //         if let Some(edit) = &action.edit {
// //             for (file, edits) in &edit.changes {
// //                 let mut content = std::fs::read_to_string(file)?;

// //                 // Apply edits in reverse order to preserve positions
// //                 for edit in edits.iter().rev() {
// //                     let start_line = edit.range.start.line as usize;
// //                     let start_char = edit.range.start.character as usize;
// //                     let end_line = edit.range.end.line as usize;
// //                     let end_char = edit.range.end.character as usize;

// //                     // Get the content up to the start position
// //                     let start_idx = if start_line == 0 {
// //                         start_char
// //                     } else {
// //                         content
// //                             .lines()
// //                             .take(start_line)
// //                             .map(|line| line.len() + 1)
// //                             .sum::<usize>()
// //                             + start_char
// //                     };

// //                     // Get the content up to the end position
// //                     let end_idx = if end_line == 0 {
// //                         end_char
// //                     } else {
// //                         content
// //                             .lines()
// //                             .take(end_line)
// //                             .map(|line| line.len() + 1)
// //                             .sum::<usize>()
// //                             + end_char
// //                     };

// //                     // Replace the content
// //                     if start_idx <= content.len() {
// //                         content = format!(
// //                             "{}{}{}",
// //                             &content[..start_idx],
// //                             &edit.new_text,
// //                             if end_idx <= content.len() {
// //                                 &content[end_idx..]
// //                             } else {
// //                                 ""
// //                             }
// //                         );
// //                     }
// //                 }

// //                 std::fs::write(file, content)?;
// //             }

// //             // Add modified files back to git index
// //             let paths: Vec<&str> = edit.changes.keys().map(|p| p.as_str()).collect();
// //             repo.add(&paths)?;
// //         }
// //     }
// //     Ok(())
// // }

// async fn get_file_lock(path: &Path) -> Arc<RwLock<()>> {
//     static FILE_LOCKS: LazyLock<Mutex<IndexMap<PathBuf, Arc<RwLock<()>>>>> =
//         LazyLock::new(Default::default);
//     let mut locks = FILE_LOCKS.lock().await;
//     let lock = locks
//         .entry(path.to_path_buf())
//         .or_insert_with(|| Arc::new(RwLock::new(())));
//     lock.clone()
// }

// #[derive(Default)]
// struct StepContext {
//     config: Config,
//     staged_files: Vec<PathBuf>,
//     lock_all: RwLock<()>,
// }
