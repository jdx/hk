use indexmap::IndexMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::{
    Result, git_util,
    step::{RenderedCommand, RunType, Step, argv_runner},
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
    command: &RenderedCommand,
    stdin: &Option<String>,
) -> Result<(String, String, i32)> {
    let rendered_step_env = step
        .env
        .iter()
        .map(|(key, value)| Ok((key.clone(), tera::render(value, tctx)?)))
        .collect::<Result<Vec<_>>>()?;
    let env_value = |name: &str| {
        test.env
            .iter()
            .rev()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .or_else(|| {
                rendered_step_env
                    .iter()
                    .rev()
                    .find(|(key, _)| key.eq_ignore_ascii_case(name))
                    .map(|(_, value)| value.as_str())
            })
    };
    let mut runner = match command {
        RenderedCommand::Argv(argv) => {
            argv_runner(argv, base_dir, env_value("PATH"), env_value("PATHEXT"))?
        }
        RenderedCommand::Shell(cmd_str) => {
            let runner = if let Some(shell) = &step.shell {
                let shell = shell.to_string();
                let mut parts = shell.split_whitespace();
                let bin = parts.next().unwrap_or("sh");
                CmdLineRunner::new(bin).args(parts)
            } else {
                CmdLineRunner::new("sh").arg("-o").arg("errexit").arg("-c")
            };
            runner.arg(cmd_str)
        }
    };
    if let Some(stdin) = stdin {
        let rendered_stdin = tera::render(stdin, tctx)?;
        runner = runner.stdin_string(rendered_stdin);
    }
    runner = runner.current_dir(base_dir);
    for (k, v) in rendered_step_env {
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

/// The files a test's command runs against, or the reason none are left.
#[derive(Debug, PartialEq, Eq)]
enum TestFiles {
    Selected(Vec<PathBuf>),
    /// The step's file filters excluded every file the test wrote.
    FiltersExcludedAll {
        written: usize,
    },
}

fn filters_excluded_all_reason(written: usize) -> String {
    format!(
        "the step's file filters excluded all {written} file(s) written by this test; set the test's `files` explicitly, or reset `tests` when overriding a builtin's `glob`"
    )
}

/// Resolve which files a test runs against.
///
/// When a test sets `files` explicitly the list is used verbatim. Otherwise the files the test
/// wrote are run through the step's filters, mirroring how a real run builds jobs. If the step has
/// filters and they discard every written file, the command would run with no files at all and fail
/// in a tool-specific way, so report the real cause instead.
fn select_test_files(step: &Step, test: &StepTest, files: Vec<PathBuf>) -> Result<TestFiles> {
    if test.files.is_some() {
        return Ok(TestFiles::Selected(files));
    }
    let written = files.len();
    let filtered = step.filter_files(&files)?;
    if filtered.is_empty() && written > 0 && step.has_filters() {
        return Ok(TestFiles::FiltersExcludedAll { written });
    }
    Ok(TestFiles::Selected(filtered))
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
    let files: Vec<PathBuf> = match &test.files {
        Some(files) => files
            .iter()
            .map(|f| tera::render(f, &tctx).unwrap_or_else(|_| f.clone()))
            .map(PathBuf::from)
            .collect(),
        None => rendered_write.keys().cloned().collect(),
    };

    // Decide whether to use a sandbox based on the explicit `tmpdir` setting,
    // or auto-detect based on whether files reference {{tmp}}.
    // If not using sandbox, operate from the project root instead.
    let uses_sandbox = test
        .tmpdir
        .unwrap_or_else(|| files.iter().any(|p| p.starts_with(&sandbox)));

    let files = match select_test_files(step, test, files)? {
        TestFiles::Selected(files) => files,
        TestFiles::FiltersExcludedAll { written } => {
            return Ok(TestResult {
                step: step.name.clone(),
                name: name.to_string(),
                ok: false,
                stdout: String::new(),
                stderr: String::new(),
                code: 0,
                duration_ms: started_at.elapsed().as_millis(),
                reasons: vec![filters_excluded_all_reason(written)],
            });
        }
    };

    let base_dir = if uses_sandbox {
        sandbox.to_path_buf()
    } else {
        git_util::find_work_tree_root()
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
    let abs_files = files
        .clone()
        .into_iter()
        .map(|f| base_dir.join(&f))
        .collect::<Vec<_>>();

    // Handle `workspace_indicator`
    if let Some(workspaces) = step.workspaces_for_files(&abs_files)? {
        let workspace_indicator = match workspaces.len() {
            0 => {
                eyre::bail!("{}: no workspace_indicator found for files", step.name,);
            }
            1 => workspaces.into_iter().next().unwrap(),
            n => {
                eyre::bail!(
                    "{}: expected exactly one workspace_indicator, found {}: {:?}",
                    step.name,
                    n,
                    workspaces
                );
            }
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

    let Some(run_cmd) = step.run_cmd(run_type).filter(|command| !command.is_empty()) else {
        eyre::bail!("{}: no command for test", step.name);
    };
    let run = run_cmd.render(&tctx, step.prefix.as_ref())?;

    // Run pre-command (before)
    let mut before_stdout = String::new();
    let mut before_stderr = String::new();
    if let Some(cmd_str) = &test.before {
        let rendered = tera::render(cmd_str, &tctx)?;
        let command = RenderedCommand::Shell(rendered);
        let (stdout, stderr, code) =
            execute_cmd(step, &tctx, &base_dir, test, &command, &None).await?;
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
        let command = RenderedCommand::Shell(rendered);
        let (a_stdout, a_stderr, a_code) =
            execute_cmd(step, &tctx, &base_dir, test, &command, &None).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::Pattern;

    fn step_with_glob(glob: &str) -> Step {
        Step {
            glob: Some(Pattern::Globs(vec![glob.to_string()])),
            ..Default::default()
        }
    }

    fn test_writing(files: &[&str]) -> StepTest {
        StepTest {
            write: files
                .iter()
                .map(|f| (f.to_string(), String::new()))
                .collect(),
            ..Default::default()
        }
    }

    fn paths(files: &[&str]) -> Vec<PathBuf> {
        files.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn filters_excluding_every_written_file_are_reported() {
        let step = step_with_glob("*.yaml");
        let test = test_writing(&["main.tf"]);

        assert_eq!(
            select_test_files(&step, &test, paths(&["main.tf"])).unwrap(),
            TestFiles::FiltersExcludedAll { written: 1 }
        );
    }

    #[test]
    fn explicit_files_bypass_filtering() {
        let step = step_with_glob("*.yaml");
        let test = StepTest {
            files: Some(vec!["main.tf".to_string()]),
            ..test_writing(&["main.tf"])
        };

        assert_eq!(
            select_test_files(&step, &test, paths(&["main.tf"])).unwrap(),
            TestFiles::Selected(paths(&["main.tf"]))
        );
    }

    #[test]
    fn step_without_filters_keeps_written_files() {
        let step = Step::default();
        let test = test_writing(&["main.tf"]);

        assert!(!step.has_filters());
        assert_eq!(
            select_test_files(&step, &test, paths(&["main.tf"])).unwrap(),
            TestFiles::Selected(paths(&["main.tf"]))
        );
    }

    #[test]
    fn test_writing_no_files_is_not_reported() {
        let step = step_with_glob("*.yaml");
        let test = StepTest::default();

        assert_eq!(
            select_test_files(&step, &test, vec![]).unwrap(),
            TestFiles::Selected(vec![])
        );
    }

    #[test]
    fn matching_files_are_filtered_as_before() {
        let step = step_with_glob("*.yaml");
        let test = test_writing(&["a.yaml", "b.tf"]);

        assert_eq!(
            select_test_files(&step, &test, paths(&["a.yaml", "b.tf"])).unwrap(),
            TestFiles::Selected(paths(&["a.yaml"]))
        );
    }

    #[test]
    fn reason_names_the_cause_and_the_fix() {
        assert_eq!(
            filters_excluded_all_reason(1),
            "the step's file filters excluded all 1 file(s) written by this test; set the test's `files` explicitly, or reset `tests` when overriding a builtin's `glob`"
        );
    }
}
