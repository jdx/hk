use indexmap::IndexMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::{
    Result,
    step::RunType,
    step::Step,
    step_test::{RunKind, StepTest},
    tera,
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
    tctx: &tera::Context,
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
        let rendered_stdin = tera::render(stdin, tctx)?;
        runner = runner.stdin_string(rendered_stdin);
    }
    runner = runner.arg(cmd_str).current_dir(base_dir);
    for (k, v) in &step.env {
        let v = tera::render(v, tctx)?;
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

fn check_exit_code(actual: i32, expected: i32) -> Option<String> {
    if actual != expected {
        Some(format!("exit code {} != expected {}", actual, expected))
    } else {
        None
    }
}

fn check_after_fail(after_fail: &Option<(i32, String, String)>) -> Option<String> {
    if let Some((code, _, _)) = after_fail {
        Some(format!("after failed with code {}", code))
    } else {
        None
    }
}

fn check_stdout_contains(stdout: &str, expected: &Option<String>) -> Option<String> {
    if let Some(needle) = expected
        && !stdout.contains(needle)
    {
        return Some(format!("stdout missing: {}", needle));
    }
    None
}

fn check_stderr_contains(stderr: &str, expected: &Option<String>) -> Option<String> {
    if let Some(needle) = expected
        && !stderr.contains(needle)
    {
        return Some(format!("stderr missing: {}", needle));
    }
    None
}

fn check_file_contents(
    expected_files: &IndexMap<String, String>,
    tctx: &tera::Context,
    base_dir: &Path,
) -> Result<Vec<String>> {
    let mut reasons = Vec::new();
    for (rel, expected) in expected_files {
        let rendered = tera::render(rel, tctx)?;
        let path = {
            let p = PathBuf::from(&rendered);
            if p.is_absolute() {
                p
            } else {
                base_dir.join(&rendered)
            }
        };
        let contents = xx::file::read_to_string(&path)?;
        if contents != *expected {
            let udiff = crate::diff::render_unified_diff(expected, &contents, "expected", "actual");
            reasons.push(format!("file mismatch: {}\n{}", path.display(), udiff));
        }
    }
    Ok(reasons)
}

pub async fn run_test_named(step: &Step, name: &str, test: &StepTest) -> Result<TestResult> {
    let started_at = Instant::now();
    let tmp = tempfile::tempdir().unwrap();
    let sandbox = tmp
        .path()
        .canonicalize()
        .unwrap_or_else(|_| tmp.path().to_path_buf());
    let mut tctx = crate::tera::Context::default();
    tctx.insert("tmp", &sandbox.display().to_string());

    let rendered_write: IndexMap<PathBuf, &String> = test
        .write
        .iter()
        .map(|(f, contents)| {
            (
                tera::render(f, &tctx).unwrap_or_else(|_| f.clone()).into(),
                contents,
            )
        })
        .collect();
    let mut files: Vec<PathBuf> = match &test.files {
        Some(files) => files
            .iter()
            .map(|f| tera::render(f, &tctx).unwrap_or_else(|_| f.clone()))
            .map(PathBuf::from)
            .collect(),
        None => rendered_write.keys().cloned().collect(),
    };

    // Decide whether to use a sandbox based on whether files reference {{tmp}}.
    // If not, operate from the project root instead.
    let uses_sandbox = files.iter().any(|p| p.starts_with(&sandbox));

    if test.files.is_none() {
        files = step.filter_files(&files)?;
    }

    let cwd = std::env::current_dir().unwrap_or_default();
    let root = xx::file::find_up(&cwd, &[".git"])
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or(cwd);
    let base_dir = if uses_sandbox {
        sandbox.to_path_buf()
    } else {
        root
    };
    if let Some(fixture) = &test.fixture {
        let src = PathBuf::from(fixture);
        xx::file::copy_dir_all(&src, &base_dir)?;
    }
    for (p, contents) in &rendered_write {
        let path = {
            if p.is_absolute() {
                p.clone()
            } else {
                base_dir.join(p)
            }
        };
        xx::file::write(&path, contents)?;
    }

    tctx.with_files(step.shell_type(), &files);

    // Handle `workspace_indicator`
    if let Some(workspaces) = step.workspaces_for_files(&files)? {
        let workspace_indicator = match workspaces.len() {
            0 => eyre::bail!("{}: no workspace_indicator found for files", step.name,),
            1 => workspaces.into_iter().next().unwrap(),
            n => eyre::bail!(
                "{}: expected exactly one workspace_indicator, found {}: {:?}",
                step.name,
                n,
                workspaces
            ),
        };

        tctx.with_workspace_indicator(&workspace_indicator);
        let workspace_dir = workspace_indicator
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        tctx.with_workspace_files(step.shell_type(), workspace_dir, &files);
    }

    // Render command
    let run_type = match test.run {
        RunKind::Fix => RunType::Fix,
        RunKind::Check => RunType::Check,
    };

    let Some(mut run) = step.run_cmd(run_type).map(|s| s.to_string()) else {
        eyre::bail!("{}: no command for test", step.name);
    };
    if let Some(prefix) = &step.prefix {
        run = format!("{prefix} {run}");
    }
    let run = tera::render(&run, &tctx)?;

    // Run pre-command (before)
    let mut before_stdout = String::new();
    let mut before_stderr = String::new();
    if let Some(cmd_str) = &test.before {
        let rendered = tera::render(cmd_str, &tctx)?;
        let (stdout, stderr, code) =
            execute_cmd(step, &tctx, &base_dir, test, &rendered, &None).await?;
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
        execute_cmd(step, &tctx, &base_dir, test, &run, &step.stdin).await?;

    // Run post-command (after) before evaluating expectations so it can contribute to assertions
    let mut after_fail: Option<(i32, String, String)> = None;
    if let Some(cmd_str) = &test.after {
        let rendered = tera::render(cmd_str, &tctx)?;
        let (a_stdout, a_stderr, a_code) =
            execute_cmd(step, &tctx, &base_dir, test, &rendered, &None).await?;
        if a_code != 0 {
            after_fail = Some((a_code, a_stdout, a_stderr));
        }
    }

    // Evaluate expectations
    let mut reasons: Vec<String> = Vec::new();
    reasons.extend(check_exit_code(code, test.expect.code));
    reasons.extend(check_after_fail(&after_fail));
    reasons.extend(check_stdout_contains(&stdout, &test.expect.stdout));
    reasons.extend(check_stderr_contains(&stderr, &test.expect.stderr));
    reasons.extend(check_file_contents(&test.expect.files, &tctx, &base_dir)?);

    // TODO: Consider adding a user-defined "cleanup" script in hk.pkl that tests can use
    // to clean up after themselves. The previous automatic cleanup caused race conditions
    // when tests ran in parallel and shared parent directories.

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
        ok: reasons.is_empty(),
        stdout: final_stdout,
        stderr: final_stderr,
        code,
        duration_ms: started_at.elapsed().as_millis(),
        reasons,
    })
}
