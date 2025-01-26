use crate::hook::Hook;
use crate::tera;
use crate::{config::Config, plugins::plugin::Plugin};
use crate::{git::Git, Result};
use globset::{Glob, GlobSetBuilder};
use indexmap::IndexMap;
use itertools::Itertools;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    thread,
};
use tokio::{
    runtime::Handle,
    sync::{Mutex, RwLock},
};

/// Sets up git hooks to run angler
#[derive(Debug, clap::Args)]
#[clap(visible_alias = "pc")]
pub struct PreCommit {}

impl PreCommit {
    pub async fn run(&self) -> Result<()> {
        let config = Config::read(Path::new("angler.toml"))?;
        let mut repo = Git::new()?;
        repo.stash_unstaged()?;
        let result = self.run_all_hooks(&mut repo, &config).await;
        if let Err(err) = repo.pop_stash() {
            error!("Failed to pop stash: {}", err);
        }
        result
    }

    async fn run_all_hooks(&self, repo: &mut Git, config: &Config) -> Result<()> {
        let staged_files = repo.staged_files()?;
        let ctx = Arc::new(HookContext {
            staged_files,
            config: config.clone(),
            ..Default::default()
        });
        dbg!(&ctx.staged_files);
        let errors = Arc::new(Mutex::new(Vec::new()));
        thread::scope(|s| {
            for hook in ctx.config.pre_commit.clone() {
                let ctx = ctx.clone();
                let handle = Handle::current();
                let mut errors = errors.clone();
                s.spawn(move || {
                    handle.block_on(async move {
                        if let Some(glob) = &hook.glob {
                            let matches = match get_glob_matches(glob, &ctx.staged_files) {
                                Ok(matches) => matches,
                                Err(err) => {
                                    errors.lock().await.push(err);
                                    return;
                                }
                            };
                            if !matches.is_empty() {
                                if let Err(err) = run_hook(&hook, &matches, &ctx).await {
                                    errors.lock().await.push(err);
                                }
                            }
                        } else if let Err(err) = run_hook(&hook, &ctx.staged_files, &ctx).await {
                            errors.lock().await.push(err);
                        }
                    });
                    dbg!("done");
                });
            }
        });
        dbg!("done");
        Ok(())
    }
}

async fn run_hook<P: AsRef<Path>>(hook: &Hook, matches: &[P], ctx: &HookContext) -> Result<()> {
    let matches = matches
        .iter()
        .map(|m| m.as_ref().to_path_buf())
        .collect_vec();
    let _lock_all = ctx.lock_all.read().await;
    let mut locks = IndexMap::new();
    let mut locks_read = IndexMap::new();
    for p in &matches {
        let file_lock = get_file_lock(p).await;
        locks.insert(p, file_lock.clone());
    }
    for (p, lock) in &locks {
        locks_read.insert(p.to_path_buf(), lock.read().await);
    }
    let mpr = ensembler::MultiProgressReport::get();
    let pr = Arc::new(mpr.add(&hook.name));
    if let Some(run) = &hook.list_files_with_errors {
        let mut ctx = tera::Context::default();
        let matches_ref: Vec<&Path> = matches.iter().map(|p| p.as_ref()).collect();
        ctx.with_staged_files(&matches_ref);
        let run = tera::render(run, &ctx)?;
        let out = ensembler::CmdLineRunner::new("sh")
            .arg("-c")
            .arg(run)
            .with_pr(pr.clone())
            .execute()?;
        let files_with_errors = out
            .stdout
            .split('\n')
            .map(|s| PathBuf::from(s.trim()))
            .collect_vec();
        if !files_with_errors.is_empty() {
            pr.set_message(format!(
                "Fixing {} files with errors",
                files_with_errors.len()
            ));
            let mut locks = IndexMap::new();
            for p in &files_with_errors {
                let file_lock = get_file_lock(p).await;
                locks.insert(p, file_lock.clone());
            }
            let mut ctx = tera::Context::default();
            ctx.with_files(&files_with_errors);
            let fix = tera::render(hook.fix.as_deref().unwrap(), &ctx)?;
            ensembler::CmdLineRunner::new("sh")
                .arg("-c")
                .arg(fix)
                .with_pr(pr.clone())
                .execute()?;
            // TODO: re-use existing repo for perf
            let mut repo = Git::new()?;
            repo.add(
                &files_with_errors
                    .iter()
                    .map(|p| p.to_str().unwrap())
                    .collect_vec(),
            )?;
            dbg!("FIXED");
        }
    } else if let Some(plugin) = hook.plugin.clone() {
        let plugin = Plugin::from(plugin);
        plugin.run().await?;
    }
    Ok(())
}

fn get_glob_matches<'a>(glob: &[String], staged_files: &'a [PathBuf]) -> Result<Vec<&'a Path>> {
    let mut gb = GlobSetBuilder::new();
    for g in glob {
        gb.add(Glob::new(g)?);
    }
    let gs = gb.build()?;
    let matches = staged_files
        .iter()
        .map(|f| f.as_ref())
        .filter(|f| gs.is_match(f))
        .collect_vec();
    Ok(matches)
}

async fn get_file_lock(path: &Path) -> Arc<RwLock<()>> {
    static FILE_LOCKS: LazyLock<Mutex<IndexMap<PathBuf, Arc<RwLock<()>>>>> =
        LazyLock::new(Default::default);
    let mut locks = FILE_LOCKS.lock().await;
    let lock = locks
        .entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(RwLock::new(())));
    lock.clone()
}

#[derive(Default)]
struct HookContext {
    config: Config,
    staged_files: Vec<PathBuf>,
    lock_all: RwLock<()>,
}
