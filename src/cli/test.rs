use crate::{Result, config::Config};
use clap::Args;
use indexmap::IndexSet;
use tokio::sync::Semaphore;

/// Run step-defined tests
#[derive(Args)]
pub struct Test {
    /// List tests without running
    #[clap(long)]
    list: bool,

    /// Filter by test name (repeatable)
    #[clap(long, value_name = "NAME", num_args = 1..)]
    name: Vec<String>,

    /// Filter by step name (repeatable)
    #[clap(long, value_name = "STEP", num_args = 1..)]
    step: Vec<String>,
}

impl Test {
    pub async fn run(self) -> Result<()> {
        let cfg = Config::get()?;
        let mut count = 0usize;
        let mut to_run: Vec<(
            String,
            crate::step::Step,
            String,
            crate::step_test::StepTest,
        )> = vec![];
        let mut seen: IndexSet<String> = IndexSet::new();
        for (_hook_name, hook) in cfg.hooks {
            for (step_name, sog) in hook.steps {
                let step = match sog {
                    crate::hook::StepOrGroup::Step(s) => s,
                    crate::hook::StepOrGroup::Group(_) => continue,
                };
                if !self.step.is_empty() && !self.step.contains(&step_name) {
                    continue;
                }
                for (tname, test) in &step.tests {
                    if !self.name.is_empty() && !self.name.contains(tname) {
                        continue;
                    }
                    // Deduplicate identical step+test pairs across hooks
                    let step_sig = serde_json::to_string(&*step).unwrap_or_default();
                    let test_sig = serde_json::to_string(&test).unwrap_or_default();
                    let sig = format!("{}::{}::{}::{}", step_name, tname, step_sig, test_sig);
                    if seen.insert(sig) {
                        if self.list {
                            println!("{step_name} :: {tname}");
                        }
                        count += 1;
                        to_run.push((
                            step_name.clone(),
                            (*step).clone(),
                            tname.clone(),
                            test.clone(),
                        ));
                    }
                }
            }
        }
        if self.list {
            println!("total: {count}");
            return Ok(());
        }
        // Execute tests in parallel up to configured jobs
        let jobs = crate::settings::Settings::try_get()?.jobs().get();
        let semaphore = std::sync::Arc::new(Semaphore::new(jobs));
        let mut handles = vec![];
        for (step_name, step, test_name, test) in to_run {
            let sem = semaphore.clone();
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire_owned().await.unwrap();
                let r = crate::test_runner::run_test_named(&step, &test_name, &test).await;
                (step_name, test_name, r)
            }));
        }
        let mut failures = 0usize;
        for h in handles {
            let (step_name, test_name, res) = h.await.unwrap();
            match res {
                Ok(r) if r.ok => println!("ok - {step_name} :: {test_name} ({}ms)", r.duration_ms),
                Ok(r) => {
                    failures += 1;
                    eyre::ensure!(!r.reasons.is_empty(), "reasons are empty");
                    eprintln!(
                        "not ok - {step_name} :: {test_name} (code={}; {}ms)\n  reasons: {}",
                        r.code,
                        r.duration_ms,
                        r.reasons.join(", ")
                    );
                    eprintln!("  stdout:\n{}", r.stdout);
                    eprintln!("  stderr:\n{}", r.stderr);
                }
                Err(e) => {
                    failures += 1;
                    eprintln!("not ok - {step_name} :: {test_name} ({e})");
                }
            }
        }
        if failures > 0 {
            eyre::bail!("{failures} test(s) failed");
        }
        Ok(())
    }
}
