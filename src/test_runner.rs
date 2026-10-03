use indexmap::IndexMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::{
    Result,
    diagnostics::{self, Diagnostic},
    git_util,
    step::{RenderedCommand, RunType, Step, argv_runner, split_shell},
    step_test::{RunKind, StepTest, StepTestDiagnostic},
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
    pub code: Option<i32>,
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
    let (stdout, stderr, code, _) =
        execute_cmd_combined(step, tctx, base_dir, test, command, stdin).await?;
    Ok((stdout, stderr, code))
}

/// Like `execute_cmd`, and also returns stdout and stderr interleaved as hk's own check runs
/// capture them, which is what a step's `diagnostic_format` parses.
async fn execute_cmd_combined(
    step: &Step,
    tctx: &tera::Context,
    base_dir: &Path,
    test: &StepTest,
    command: &RenderedCommand,
    stdin: &Option<String>,
) -> Result<(String, String, i32, String)> {
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
                let parts = split_shell(&shell.to_string());
                let bin = parts.first().map_or("sh", String::as_str);
                CmdLineRunner::new_direct(bin).args(parts.iter().skip(1))
            } else {
                CmdLineRunner::new_direct("sh")
                    .arg("-o")
                    .arg("errexit")
                    .arg("-c")
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
    let (stdout, stderr, code, combined) = match result {
        Ok(r) => (
            r.stdout,
            r.stderr,
            r.status.code().unwrap_or(0),
            r.combined_output,
        ),
        Err(e) => {
            if let ensembler::Error::ScriptFailed(tuple) = &e {
                let r = &tuple.3;
                (
                    r.stdout.clone(),
                    r.stderr.clone(),
                    r.status.code().unwrap_or(1),
                    r.combined_output.clone(),
                )
            } else {
                return Err(e.into());
            }
        }
    };
    Ok((stdout, stderr, code, combined))
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

fn check_diff_not_applied_reason(code: i32) -> String {
    format!(
        "check_diff exited {code} but hk could not apply its output as a patch; check_diff must print a unified diff naming each file"
    )
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

/// No leading `./` and no trailing `/`. On Windows, where `\` separates directories, also
/// forward slashes and no verbatim prefix (`\\?\`) and lower case, because its paths are case
/// insensitive. Elsewhere `\` is an ordinary filename character and case matters.
fn normalize_path(path: &str, windows: bool) -> String {
    let mut path = path.to_string();
    if windows {
        path = path.replace('\\', "/").to_lowercase();
        if let Some(rest) = path.strip_prefix("//?/") {
            path = rest.to_string();
        }
    }
    let path = path.strip_prefix("./").unwrap_or(&path);
    if path.len() > 1 {
        path.trim_end_matches('/').to_string()
    } else {
        path.to_string()
    }
}

fn is_absolute_path(path: &str, windows: bool) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || (windows && bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &path[1..3] == ":/")
}

/// Canonicalize the nearest existing ancestor of `path` and re-append the components that don't
/// exist (a file the tool or the test later removed), so a symlinked directory resolves even when
/// the file under it is gone.
fn canonicalize_existing(path: &Path) -> Option<PathBuf> {
    let mut rest = Vec::new();
    let mut ancestor = path;
    loop {
        if let Ok(canonical) = ancestor.canonicalize() {
            return Some(rest.iter().rev().fold(canonical, |p, c| p.join(c)));
        }
        rest.push(ancestor.file_name()?);
        ancestor = ancestor.parent()?;
    }
}

/// The spellings of the directory a test runs its command in: as given and canonicalized, so a
/// symlinked sandbox (such as macOS's `/var` for `/private/var`) is the same directory either way.
fn sandbox_roots(base: &Path, windows: bool) -> Vec<String> {
    let mut roots = vec![normalize_path(&base.display().to_string(), windows)];
    if let Some(canonical) = canonicalize_existing(base) {
        roots.push(normalize_path(&canonical.display().to_string(), windows));
    }
    roots.retain(|root| is_absolute_path(root, windows));
    roots.dedup();
    roots
}

/// Whether the diagnostic path a tool printed names the file `expected`. A relative printed path
/// must equal `expected`. An absolute printed path matches only if it, or its canonicalized form
/// when the file exists, lies inside one of the sandbox `roots` and the rest equals `expected`;
/// an absolute path elsewhere never matches a relative expectation.
fn same_path(printed: &str, expected: &str, roots: &[String], windows: bool) -> bool {
    let expected = normalize_path(expected, windows);
    let mut forms = vec![normalize_path(printed, windows)];
    if forms[0] == expected {
        return true;
    }
    if !is_absolute_path(&forms[0], windows) || is_absolute_path(&expected, windows) {
        return false;
    }
    if let Some(canonical) = canonicalize_existing(Path::new(printed)) {
        forms.push(normalize_path(&canonical.display().to_string(), windows));
    }
    forms.iter().any(|printed| {
        roots.iter().any(|root| {
            printed
                .strip_prefix(root.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                == Some(expected.as_str())
        })
    })
}

fn diagnostic_matches(
    diagnostic: &Diagnostic,
    expected: &StepTestDiagnostic,
    roots: &[String],
) -> bool {
    let start = diagnostic.range.as_ref().map(|range| &range.start);
    expected.path.as_ref().is_none_or(|path| {
        diagnostic
            .path
            .as_deref()
            .is_some_and(|actual| same_path(actual, path, roots, cfg!(windows)))
    }) && expected
        .line
        .is_none_or(|line| start.is_some_and(|start| start.line == line))
        && expected
            .column
            .is_none_or(|column| start.is_some_and(|start| start.column == column))
        && expected
            .severity
            .as_ref()
            .is_none_or(|severity| *severity == diagnostic.severity)
        && expected
            .rule
            .as_ref()
            .is_none_or(|rule| diagnostic.rule.as_ref() == Some(rule))
        && expected
            .message
            .as_ref()
            .is_none_or(|message| diagnostic.message.contains(message))
}

fn describe(diagnostic: &Diagnostic) -> String {
    let (line, column) = diagnostic
        .range
        .as_ref()
        .map(|range| (range.start.line, range.start.column))
        .unzip();
    format!(
        "{}:{}:{} {:?} {:?} {:?}",
        diagnostic.path.as_deref().unwrap_or("-"),
        line.map_or("-".to_string(), |line| line.to_string()),
        column.map_or("-".to_string(), |column| column.to_string()),
        diagnostic.severity,
        diagnostic.rule,
        diagnostic.message
    )
}

/// Parse a `check` test's combined output with the step's `diagnostic_format`, as a real check
/// does for structured output and SARIF, and report each expected diagnostic that no parsed
/// diagnostic matches.
fn check_diagnostics(step: &Step, test: &StepTest, combined: &str, sandbox: &Path) -> Vec<String> {
    let expected = &test.expect.diagnostics;
    if expected.is_empty() {
        return vec![];
    }
    if !matches!(test.run, RunKind::Check) {
        return vec!["expect.diagnostics requires run = \"check\"".to_string()];
    }
    let Some(format) = step.diagnostic_format else {
        return vec!["expect.diagnostics requires the step to set diagnostic_format".to_string()];
    };
    // A real check feeds the parser more than this one invocation's output in two cases the
    // test can't reproduce, so a passing assertion could misrepresent `hk check --sarif`.
    if step.check_failed_files && (step.check_diff.is_some() || step.check_list_files.is_some()) {
        return vec![
            "expect.diagnostics can't model a check-first step (check_failed_files with check_diff or check_list_files): a real check captures the file-reporting command's output too before parsing"
                .to_string(),
        ];
    }
    if matches!(
        format,
        crate::step::DiagnosticFormat::Sarif | crate::step::DiagnosticFormat::EslintJson
    ) && (step.batch || step.workspace_indicator.is_some())
    {
        return vec![
            "expect.diagnostics can't model a batched or workspace step with a single-document diagnostic_format (sarif, eslint-json): a real check joins every job's output before parsing, so separate documents would not parse"
                .to_string(),
        ];
    }
    let tool = step.diagnostic_tool.as_deref().unwrap_or(&step.name);
    let parsed = diagnostics::parse(format, &step.name, tool, combined);
    let roots = sandbox_roots(sandbox, cfg!(windows));
    expected
        .iter()
        .filter(|expected| {
            !parsed
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic_matches(diagnostic, expected, &roots))
        })
        .map(|expected| {
            format!(
                "no diagnostic matches {expected:?}; parsed {} diagnostic(s): [{}]; parse warnings: {:?}",
                parsed.diagnostics.len(),
                parsed
                    .diagnostics
                    .iter()
                    .map(describe)
                    .collect::<Vec<_>>()
                    .join("; "),
                parsed.warnings
            )
        })
        .collect()
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
                code: None,
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
    let run_cmd = match test.run {
        RunKind::Fix => step.run_cmd(RunType::Fix),
        RunKind::Check => step.run_cmd(RunType::Check),
        RunKind::Diff => step.check_diff.as_ref(),
    };
    let Some(run_cmd) = run_cmd.filter(|command| !command.is_empty()) else {
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
                code: Some(code),
                duration_ms: started_at.elapsed().as_millis(),
                reasons: vec![format!("before failed with code {}", code)],
            });
        }
    }

    // Run main command

    let (mut stdout, mut stderr, mut code, combined) =
        execute_cmd_combined(step, &tctx, &base_dir, test, &run, &step.stdin).await?;

    // A diff test applies the patch as fix mode would, but a patch that
    // doesn't apply fails the test instead of falling back to `fix`.
    let mut diff_reason = None;
    if matches!(test.run, RunKind::Diff) && code != 0 {
        if step.apply_diff_output(&stdout, base_dir.to_str())? {
            code = 0;
            if step.check_after_diff
                && let Some(check) = step.check.as_ref().filter(|c| !c.is_empty())
            {
                let check = check.render(&tctx, step.prefix.as_ref())?;
                let (c_stdout, c_stderr, c_code) =
                    execute_cmd(step, &tctx, &base_dir, test, &check, &step.stdin).await?;
                stdout = format!("{stdout}\n[check]\n{c_stdout}");
                stderr = format!("{stderr}\n[check]\n{c_stderr}");
                code = c_code;
            }
        } else {
            diff_reason = Some(check_diff_not_applied_reason(code));
        }
    }

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
    reasons.extend(diff_reason);
    reasons.extend(check_exit_code(code, test.expect.code));
    reasons.extend(check_after_fail(&after_fail));
    reasons.extend(check_stdout_contains(&stdout, &test.expect.stdout));
    reasons.extend(check_stderr_contains(&stderr, &test.expect.stderr));
    reasons.extend(check_file_contents(&test.expect.files, &tctx, &base_dir)?);
    reasons.extend(check_diagnostics(step, test, &combined, &base_dir));

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
        code: Some(code),
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

    fn gcc_step() -> Step {
        Step {
            name: "lint".to_string(),
            diagnostic_format: Some(crate::step::DiagnosticFormat::Gcc),
            ..Default::default()
        }
    }

    fn expecting(diagnostics: Vec<StepTestDiagnostic>) -> StepTest {
        StepTest {
            expect: crate::step_test::StepTestExpect {
                diagnostics,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn expected_diagnostics_match_parsed_output() {
        let test = expecting(vec![StepTestDiagnostic {
            path: Some("a.c".to_string()),
            line: Some(2),
            column: Some(4),
            severity: Some(diagnostics::Severity::Warning),
            rule: Some("W1".to_string()),
            message: Some("first".to_string()),
        }]);

        assert!(
            check_diagnostics(
                &gcc_step(),
                &test,
                "./a.c:2:4: warning: first line [W1]\n  context\n",
                Path::new(".")
            )
            .is_empty()
        );
    }

    #[test]
    fn absolute_paths_match_only_inside_the_sandbox() {
        let roots = vec!["/tmp/sandbox".to_string()];
        let same = |printed, expected| same_path(printed, expected, &roots, false);
        assert!(same("/tmp/sandbox/proto/a.proto", "proto/a.proto"));
        assert!(same("./a.c", "a.c"));
        // a different directory with the same tail
        assert!(!same("/other/project/proto/a.proto", "proto/a.proto"));
        assert!(!same("/tmp/sandbox2/proto/a.proto", "proto/a.proto"));
        assert!(!same("/tmp/sandbox/x/proto/a.proto", "proto/a.proto"));
        // relative printed paths stay exact
        assert!(!same("sub/a.c", "a.c"));
    }

    #[test]
    fn backslash_is_a_filename_character_off_windows() {
        let roots = vec!["/tmp/sandbox".to_string()];
        assert!(!same_path(r"a\b.c", "a/b.c", &roots, false));
        assert!(same_path(r"a\b.c", r"a\b.c", &roots, false));
        assert!(!same_path(r"/tmp/sandbox/a\b.c", "a/b.c", &roots, false));
        // and case matters
        assert!(!same_path("/tmp/sandbox/A.c", "a.c", &roots, false));
    }

    #[test]
    fn windows_paths_are_normalized_and_case_insensitive() {
        let roots = sandbox_roots(Path::new(r"C:\Tmp\Sandbox"), true);
        let same = |printed, expected| same_path(printed, expected, &roots, true);
        assert!(same(r"C:\tmp\sandbox\src\main.c", "src/main.c"));
        assert!(same(r"c:\TMP\Sandbox\Src\Main.c", "src/main.c"));
        assert!(same(r"\\?\C:\Tmp\Sandbox\src\main.c", "src/main.c"));
        assert!(same(r"src\main.c", "src/main.c"));
        assert!(!same(r"C:\other\src\main.c", "src/main.c"));
    }

    #[cfg(windows)]
    #[test]
    fn this_platform_normalizes_windows_paths() {
        assert!(same_path(
            r"C:\Sandbox\a.c",
            "a.c",
            &sandbox_roots(Path::new(r"c:\sandbox"), cfg!(windows)),
            cfg!(windows)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn deleted_files_in_a_symlinked_sandbox_still_match() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().canonicalize().unwrap().join("real");
        std::fs::create_dir_all(real.join("src")).unwrap();
        let link = real.parent().unwrap().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let other = real.parent().unwrap().join("other");
        std::fs::create_dir(&other).unwrap();

        // neither printed file exists (the tool or an `after` command removed them)
        let roots = sandbox_roots(&link, false);
        let gone = real.join("src/gone.c").display().to_string();
        assert!(same_path(&gone, "src/gone.c", &roots, false));
        let roots = sandbox_roots(&real, false);
        let gone = link.join("src/gone.c").display().to_string();
        assert!(same_path(&gone, "src/gone.c", &roots, false));
        // a deleted file in another directory is still different
        let elsewhere = other.join("src/gone.c").display().to_string();
        assert!(!same_path(&elsewhere, "src/gone.c", &roots, false));
        // and a sandbox that no longer exists compares by its own spelling
        assert!(canonicalize_existing(Path::new("/nonexistent-root-xyz/a/b")).is_some());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_sandbox_is_the_same_directory() {
        // macOS reaches its temporary directory through the `/var` symlink to `/private/var`;
        // canonicalizing both sides makes that, and any other symlink, compare equal.
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().canonicalize().unwrap().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("a.c"), "").unwrap();
        let link = real.parent().unwrap().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let other = real.parent().unwrap().join("other");
        std::fs::create_dir(&other).unwrap();
        std::fs::write(other.join("a.c"), "").unwrap();

        // the sandbox was reached through the link; the tool printed the real path
        let roots = sandbox_roots(&link, false);
        assert!(same_path(
            &real.join("a.c").display().to_string(),
            "a.c",
            &roots,
            false
        ));
        // the sandbox is the real directory; the tool printed the link path
        let roots = sandbox_roots(&real, false);
        assert!(same_path(
            &link.join("a.c").display().to_string(),
            "a.c",
            &roots,
            false
        ));
        // a different directory is still different
        assert!(!same_path(
            &other.join("a.c").display().to_string(),
            "a.c",
            &roots,
            false
        ));
    }

    #[test]
    fn unmatched_diagnostic_lists_what_was_parsed() {
        let test = expecting(vec![StepTestDiagnostic {
            line: Some(9),
            ..Default::default()
        }]);

        let reasons =
            check_diagnostics(&gcc_step(), &test, "a.c:2:4: error: bad\n", Path::new("."));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("parsed 1 diagnostic(s)"), "{reasons:?}");
        assert!(reasons[0].contains("a.c:2:4"), "{reasons:?}");
    }

    #[test]
    fn diagnostics_need_a_format_and_a_check_run() {
        let test = expecting(vec![StepTestDiagnostic::default()]);
        let reasons = check_diagnostics(
            &Step::default(),
            &test,
            "a.c:2:4: error: bad\n",
            Path::new("."),
        );
        assert_eq!(
            reasons,
            vec!["expect.diagnostics requires the step to set diagnostic_format"]
        );

        let fix = StepTest {
            run: RunKind::Fix,
            ..test
        };
        assert_eq!(
            check_diagnostics(&gcc_step(), &fix, "a.c:2:4: error: bad\n", Path::new(".")),
            vec!["expect.diagnostics requires run = \"check\""]
        );
    }

    #[test]
    fn check_first_steps_are_rejected() {
        let test = expecting(vec![StepTestDiagnostic::default()]);
        let step = Step {
            check_failed_files: true,
            check_list_files: Some(crate::step::Command::Shell("list".parse().unwrap())),
            ..gcc_step()
        };
        let reasons = check_diagnostics(&step, &test, "a.c:2:4: error: bad\n", Path::new("."));
        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("check-first"), "{reasons:?}");

        // check_failed_files without a file-reporting command runs only `check`
        let step = Step {
            check_failed_files: true,
            ..gcc_step()
        };
        assert!(
            check_diagnostics(&step, &test, "a.c:2:4: error: bad\n", Path::new(".")).is_empty()
        );
    }

    #[test]
    fn batched_steps_reject_single_document_formats_only() {
        let test = expecting(vec![StepTestDiagnostic::default()]);
        let sarif = r#"{"runs":[{"results":[{"message":{"text":"m"}}]}]}"#;
        for format in [
            crate::step::DiagnosticFormat::Sarif,
            crate::step::DiagnosticFormat::EslintJson,
        ] {
            let step = Step {
                batch: true,
                diagnostic_format: Some(format),
                ..gcc_step()
            };
            let reasons = check_diagnostics(&step, &test, sarif, Path::new("."));
            assert_eq!(reasons.len(), 1);
            assert!(reasons[0].contains("batched"), "{reasons:?}");
        }
        // Line-oriented formats concatenate safely across jobs.
        let step = Step {
            batch: true,
            ..gcc_step()
        };
        assert!(
            check_diagnostics(&step, &test, "a.c:2:4: error: bad\n", Path::new(".")).is_empty()
        );
        // An unbatched single-document step is parsed as a real check would.
        let step = Step {
            diagnostic_format: Some(crate::step::DiagnosticFormat::Sarif),
            ..gcc_step()
        };
        assert!(check_diagnostics(&step, &test, sarif, Path::new(".")).is_empty());
    }

    #[test]
    fn no_expectations_skip_parsing() {
        assert!(
            check_diagnostics(
                &Step::default(),
                &StepTest::default(),
                "anything",
                Path::new(".")
            )
            .is_empty()
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
