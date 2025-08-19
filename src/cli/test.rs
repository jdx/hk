use crate::{Result, config::Config};
use clap::Args;
use tokio::sync::Semaphore;

/// Run step-defined tests
#[derive(Args)]
pub struct Test {
    /// Filter by step name (repeatable)
    #[clap(long, value_name = "STEP", num_args = 1..)]
    step: Vec<String>,

    /// Filter by test name (repeatable)
    #[clap(long, value_name = "NAME", num_args = 1..)]
    name: Vec<String>,

    /// List tests without running
    #[clap(long)]
    list: bool,
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
        if self.list {
            println!("total: {count}");
            return Ok(());
        }
        // Execute tests in parallel up to configured jobs
        let jobs = crate::settings::Settings::get().jobs.get();
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
                Ok(r) if r.ok => println!("ok - {step_name} :: {test_name}"),
                Ok(r) => {
                    failures += 1;
                    println!("not ok - {step_name} :: {test_name} (code={})", r.code);
                }
                Err(e) => {
                    failures += 1;
                    println!("not ok - {step_name} :: {test_name} ({e})");
                }
            }
        }
        if failures > 0 {
            eyre::bail!("{failures} test(s) failed");
        }
        Ok(())
    }
}
