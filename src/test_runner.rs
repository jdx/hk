use std::path::PathBuf;
use std::time::Instant;

use crate::{
    Result,
    step::Step,
    step_test::{RunKind, StepTest},
};
use ensembler::CmdLineRunner;

#[allow(unused)]
pub struct TestResult {
    pub step: String,
    pub name: String,
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
    pub duration_ms: u128,
}

pub async fn run_test_named(step: &Step, name: &str, test: &StepTest) -> Result<TestResult> {
    let started_at = Instant::now();
    let tmp = tempfile::tempdir().unwrap();
    let sandbox = tmp.path().to_path_buf();
    let mut tctx = crate::tera::Context::default();
    tctx.insert("tmp", &sandbox.display().to_string());
    let files: Vec<PathBuf> = match &test.files {
        Some(files) => files
            .iter()
            .map(|f| crate::tera::render(f, &tctx).unwrap_or_else(|_| f.clone()))
            .map(PathBuf::from)
            .collect(),
        None => test
            .write
            .keys()
            .map(|f| crate::tera::render(f, &tctx).unwrap_or_else(|_| f.clone()))
            .map(PathBuf::from)
            .collect(),
    };
    tctx.with_files(step.shell_type(), &files);

    if let Some(fixture) = &test.fixture {
        let src = PathBuf::from(fixture);
        xx::file::copy_dir_all(&src, &sandbox)?;
    }
    for (rel, contents) in &test.write {
        let rendered = crate::tera::render(rel, &tctx)?;
        let path = {
            let p = PathBuf::from(&rendered);
            if p.is_absolute() {
                p
            } else {
                sandbox.join(&rendered)
            }
        };
        xx::file::write(&path, contents)?;
    }

    // Render command
    let cmd_string = match test.run {
        RunKind::Check => step
            .run_cmd(crate::step::RunType::Check(step.check_type()))
            .map(|s| s.to_string()),
        RunKind::Fix => step
            .run_cmd(crate::step::RunType::Fix)
            .map(|s| s.to_string()),
    };
    let Some(mut run) = cmd_string else {
        eyre::bail!("{}: no command for test", step.name);
    };
    if let Some(prefix) = &step.prefix {
        run = format!("{prefix} {run}");
    }
    let run = crate::tera::render(&run, &tctx)?;

    let mut cmd = if let Some(shell) = &step.shell {
        let shell = shell.to_string();
        let mut parts = shell.split_whitespace();
        let bin = parts.next().unwrap_or("sh");
        CmdLineRunner::new(bin).args(parts)
    } else {
        CmdLineRunner::new("sh").arg("-o").arg("errexit").arg("-c")
    };
    cmd = cmd.arg(&run).current_dir(&sandbox);
    // Merge env: step then test (test wins)
    for (k, v) in &step.env {
        let v = crate::tera::render(v, &tctx)?;
        cmd = cmd.env(k, v);
    }
    for (k, v) in &test.env {
        cmd = cmd.env(k, v);
    }

    let result = cmd.execute().await;
    let (stdout, stderr, code) = match result {
        Ok(r) => (r.stdout, r.stderr, r.status.code().unwrap_or(0)),
        Err(e) => {
            if let ensembler::Error::ScriptFailed(tuple) = &e {
                let r = &tuple.3;
                (
                    r.stdout.clone(),
                    r.stderr.clone(),
                    r.status.code().unwrap_or(1),
                )
            } else {
                return Err(e.into());
            }
        }
    };

    // Evaluate expectations
    let mut pass = code == test.expect.code;
    if let Some(needle) = &test.expect.stdout {
        if !stdout.contains(needle) {
            pass = false;
        }
    }
    if let Some(needle) = &test.expect.stderr {
        if !stderr.contains(needle) {
            pass = false;
        }
    }
    for (rel, expected) in &test.expect.files {
        let rendered = crate::tera::render(rel, &tctx)?;
        let path = {
            let p = PathBuf::from(&rendered);
            if p.is_absolute() {
                p
            } else {
                sandbox.join(&rendered)
            }
        };
        let contents = xx::file::read_to_string(&path)?;
        if &contents != expected {
            pass = false;
        }
    }

    Ok(TestResult {
        step: step.name.clone(),
        name: name.to_string(),
        ok: pass,
        stdout,
        stderr,
        code,
        duration_ms: started_at.elapsed().as_millis(),
    })
}
