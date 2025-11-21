use std::fs;
use std::path::{Path, PathBuf};
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
    pub reasons: Vec<String>,
}

async fn execute_cmd(
    step: &Step,
    tctx: &crate::tera::Context,
    base_dir: &Path,
    test: &StepTest,
    cmd_str: &str,
    stdin: &Option<String>,
) -> Result<(String, String, i32)> {
    let mut runner = if let Some(shell) = &step.shell {
        let shell = shell.to_string();
        let mut parts = shell.split_whitespace();
        let bin = parts.next().unwrap_or("sh");
        CmdLineRunner::new(bin).args(parts)
    } else {
        CmdLineRunner::new("sh").arg("-o").arg("errexit").arg("-c")
    };
    if let Some(stdin) = stdin {
        let rendered_stdin = crate::tera::render(stdin, &tctx)?;
        runner = runner.stdin_string(rendered_stdin);
    }
    runner = runner.arg(cmd_str).current_dir(base_dir);
    for (k, v) in &step.env {
        let v = crate::tera::render(v, tctx)?;
        runner = runner.env(k, v);
    }
    for (k, v) in &test.env {
        runner = runner.env(k, v);
    }
    let result = runner.execute().await;
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
    Ok((stdout, stderr, code))
}

pub async fn run_test_named(step: &Step, name: &str, test: &StepTest) -> Result<TestResult> {
    let started_at = Instant::now();
    let tmp = tempfile::tempdir().unwrap();
    let sandbox = tmp.path().to_path_buf();
    let mut tctx = crate::tera::Context::default();
    tctx.insert("tmp", &sandbox.display().to_string());
    // Decide whether to use a sandbox based on whether files reference {{tmp}}.
    // If not, operate from the project root instead.
    let use_sandbox = match &test.files {
        Some(files) => files.iter().any(|f| f.contains("{{tmp}}")),
        None => test.write.keys().any(|f| f.contains("{{tmp}}")),
    };
    let cwd = std::env::current_dir().unwrap_or_default();
    let root = xx::file::find_up(&cwd, &[".git"])
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or(cwd);
    let base_dir = if use_sandbox { &sandbox } else { &root };
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

    // Track created files/dirs for cleanup on success
    let mut created_files: Vec<PathBuf> = Vec::new();
    let mut created_dirs: Vec<PathBuf> = Vec::new();

    if let Some(fixture) = &test.fixture {
        let src = PathBuf::from(fixture);
        // Pre-scan to determine which files/dirs will be newly created
        let mut stack: Vec<PathBuf> = vec![src.clone()];
        while let Some(cur) = stack.pop() {
            let read_dir = match fs::read_dir(&cur) {
                Ok(iter) => iter,
                Err(_) => continue,
            };
            for entry in read_dir.flatten() {
                let p = entry.path();
                let rel = p.strip_prefix(&src).unwrap_or(&p);
                let dest = base_dir.join(rel);
                if p.is_dir() {
                    if !dest.exists() {
                        created_dirs.push(dest.clone());
                    }
                    stack.push(p);
                } else if !dest.exists() {
                    created_files.push(dest.clone());
                }
            }
        }
        xx::file::copy_dir_all(&src, base_dir)?;
    }
    for (rel, contents) in &test.write {
        let rendered = crate::tera::render(rel, &tctx)?;
        let path = {
            let p = PathBuf::from(&rendered);
            if p.is_absolute() {
                p
            } else {
                base_dir.join(&rendered)
            }
        };
        // Track newly created parent dirs and file
        if let Some(mut parent) = path.parent().map(|p| p.to_path_buf()) {
            let mut to_create: Vec<PathBuf> = Vec::new();
            while !parent.exists() {
                to_create.push(parent.clone());
                if !parent.pop() {
                    break;
                }
            }
            // Only record directories that are under base_dir
            for dir in to_create {
                if dir.starts_with(base_dir) {
                    created_dirs.push(dir);
                }
            }
        }
        if !path.exists() {
            created_files.push(path.clone());
        }
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

    // Run pre-command (before)
    let mut before_stdout = String::new();
    let mut before_stderr = String::new();
    if let Some(cmd_str) = &test.before {
        let rendered = crate::tera::render(cmd_str, &tctx)?;
        let (stdout, stderr, code) =
            execute_cmd(step, &tctx, base_dir, test, &rendered, &None).await?;
        before_stdout = stdout.clone();
        before_stderr = stderr.clone();
        if code != 0 {
            return Ok(TestResult {
                step: step.name.clone(),
                name: name.to_string(),
                ok: false,
                stdout,
                stderr,
                code,
                duration_ms: started_at.elapsed().as_millis(),
                reasons: vec![format!("before failed with code {}", code)],
            });
        }
    }

    // Run main command

    let (stdout, stderr, code) =
        execute_cmd(step, &tctx, base_dir, test, &run, &step.stdin).await?;

    // Run post-command (after) before evaluating expectations so it can contribute to assertions
    let mut after_fail: Option<(i32, String, String)> = None;
    if let Some(cmd_str) = &test.after {
        let rendered = crate::tera::render(cmd_str, &tctx)?;
        let (a_stdout, a_stderr, a_code) =
            execute_cmd(step, &tctx, base_dir, test, &rendered, &None).await?;
        if a_code != 0 {
            after_fail = Some((a_code, a_stdout, a_stderr));
        }
    }

    // Evaluate expectations
    let mut reasons: Vec<String> = Vec::new();
    let mut pass = code == test.expect.code;
    if code != test.expect.code {
        reasons.push(format!(
            "exit code {} != expected {}",
            code, test.expect.code
        ));
    }
    if let Some((a_code, _a_stdout, _a_stderr)) = after_fail {
        pass = false;
        reasons.push(format!("after failed with code {}", a_code));
    }
    if let Some(needle) = &test.expect.stdout {
        if !stdout.contains(needle) {
            pass = false;
            reasons.push(format!("stdout missing: {}", needle));
        }
    }
    if let Some(needle) = &test.expect.stderr {
        if !stderr.contains(needle) {
            pass = false;
            reasons.push(format!("stderr missing: {}", needle));
        }
    }
    for (rel, expected) in &test.expect.files {
        let rendered = crate::tera::render(rel, &tctx)?;
        let path = {
            let p = PathBuf::from(&rendered);
            if p.is_absolute() {
                p
            } else {
                base_dir.join(&rendered)
            }
        };
        let contents = xx::file::read_to_string(&path)?;
        if &contents != expected {
            pass = false;
            let udiff = render_unified_diff(expected, &contents);
            reasons.push(format!("file mismatch: {}\n{}", path.display(), udiff));
        }
    }

    // Cleanup created fixtures if test passed
    if pass {
        // Remove files first
        for f in &created_files {
            let _ = fs::remove_file(f);
        }
        // Remove directories in reverse depth order
        created_dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));
        for d in &created_dirs {
            let _ = xx::file::remove_dir_all(d);
        }
    }

    // Prepend before output to help with debugging
    let final_stdout = if before_stdout.is_empty() {
        stdout
    } else {
        format!("[before]\n{}\n[main]\n{}", before_stdout, stdout)
    };
    let final_stderr = if before_stderr.is_empty() {
        stderr
    } else {
        format!("[before]\n{}\n[main]\n{}", before_stderr, stderr)
    };

    Ok(TestResult {
        step: step.name.clone(),
        name: name.to_string(),
        ok: pass,
        stdout: final_stdout,
        stderr: final_stderr,
        code,
        duration_ms: started_at.elapsed().as_millis(),
        reasons,
    })
}

fn render_unified_diff(expected: &str, actual: &str) -> String {
    use similar::TextDiff;
    let diff = TextDiff::from_lines(expected, actual);
    diff.unified_diff()
        .context_radius(3)
        .header("expected", "actual")
        .to_string()
}
