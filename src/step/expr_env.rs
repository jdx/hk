//! Expression evaluation environment for step conditions.
//!
//! Provides the expression evaluation context used for `condition` fields
//! in step configurations. Supports custom functions like `exec()` for
//! running shell commands during condition evaluation.

use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::LazyLock;

/// Default expression evaluation context.
pub static EXPR_CTX: LazyLock<expr::Context> = LazyLock::new(expr::Context::default);

/// Expression environment with custom functions.
///
/// Currently provides:
/// - `exec(command)` - Execute a shell command and return its stdout. A
///   command that exits non-zero, or prints output that is not UTF-8, is an
///   error.
/// - `exec_ok(command)` - Execute a shell command and return whether it exited
///   with status 0. Use it in conditions: `exec_ok('test -f check.js')`.
/// - `env(name)` - Return an environment variable, or an empty string when unset
pub static EXPR_ENV: LazyLock<expr::Environment> = LazyLock::new(|| {
    let mut env = expr::Environment::new();

    env.add_function("exec", |c| {
        let script = shell_script_arg("exec", &c.args)?;
        let output = run_shell(script, Stdio::piped())?;
        if !output.status.success() {
            return Err(expr::Error::ExprError(format!(
                "exec({script:?}) {}; use exec_ok() to test whether a command succeeds",
                describe_failure(output.status)
            )));
        }
        let stdout = String::from_utf8(output.stdout).map_err(|_| {
            expr::Error::ExprError(format!(
                "exec({script:?}) printed output that is not valid UTF-8; use exec_ok() if only its exit status matters"
            ))
        })?;
        Ok(expr::Value::String(stdout))
    });

    env.add_function("exec_ok", |c| {
        let script = shell_script_arg("exec_ok", &c.args)?;
        let output = run_shell(script, Stdio::null())?;
        Ok(expr::Value::Bool(output.status.success()))
    });

    env.add_function("env", |c| {
        if c.args.len() != 1 {
            return Err(expr::Error::ExprError(
                "env() expects exactly one string argument".to_string(),
            ));
        }
        let name = c.args[0].as_string().ok_or_else(|| {
            expr::Error::ExprError("env() expects exactly one string argument".to_string())
        })?;
        Ok(expr::Value::String(std::env::var(name).unwrap_or_default()))
    });

    env
});

/// The single string argument of `exec()` and `exec_ok()`.
fn shell_script_arg<'a>(name: &str, args: &'a [expr::Value]) -> expr::Result<&'a str> {
    let script = match args {
        [arg] => arg.as_string(),
        _ => None,
    };
    script.ok_or_else(|| {
        expr::Error::ExprError(format!("{name}() expects exactly one string argument"))
    })
}

fn run_shell(script: &str, stdout: Stdio) -> expr::Result<Output> {
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdin(Stdio::inherit())
        .stdout(stdout)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| expr::Error::ExprError(format!("failed to run `sh -c {script}`: {e}")))
}

fn describe_failure(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exited with code {code}"),
        None => "was terminated by a signal".to_string(),
    }
}

/// Evaluate an hk condition while preserving expr-lang v1 string behavior.
///
/// Pkl decodes escapes before hk sees a condition, so a Pkl string containing
/// `\n` reaches the expression parser as a literal newline inside a quoted
/// string. expr-lang v1 accepted that syntax, while v2 requires the newline to
/// be escaped. Rewrite only raw newlines in interpreted string literals;
/// multiline backtick strings, comments, and expression whitespace are left
/// unchanged.
pub fn eval_condition(code: &str, ctx: &expr::Context) -> expr::Result<expr::Value> {
    EXPR_ENV.eval(&escape_quoted_newlines(code), ctx)
}

#[derive(Clone, Copy)]
enum LexState {
    Normal,
    SingleQuoted,
    DoubleQuoted,
    BacktickQuoted,
    LineComment,
    BlockComment,
}

fn escape_quoted_newlines(code: &str) -> String {
    let mut escaped = String::with_capacity(code.len());
    let mut chars = code.chars().peekable();
    let mut state = LexState::Normal;

    while let Some(ch) = chars.next() {
        match state {
            LexState::Normal => {
                escaped.push(ch);
                state = match ch {
                    '\'' => LexState::SingleQuoted,
                    '"' => LexState::DoubleQuoted,
                    '`' => LexState::BacktickQuoted,
                    '/' if chars.peek() == Some(&'/') => {
                        escaped.push(chars.next().expect("peeked line-comment delimiter"));
                        LexState::LineComment
                    }
                    '/' if chars.peek() == Some(&'*') => {
                        escaped.push(chars.next().expect("peeked block-comment delimiter"));
                        LexState::BlockComment
                    }
                    _ => LexState::Normal,
                };
            }
            LexState::SingleQuoted | LexState::DoubleQuoted => {
                let quote = if matches!(state, LexState::SingleQuoted) {
                    '\''
                } else {
                    '"'
                };
                match ch {
                    '\\' => {
                        escaped.push(ch);
                        if let Some(next) = chars.next() {
                            escaped.push(next);
                        }
                    }
                    '\n' => escaped.push_str("\\n"),
                    '\r' => escaped.push_str("\\r"),
                    _ => {
                        escaped.push(ch);
                        if ch == quote {
                            state = LexState::Normal;
                        }
                    }
                }
            }
            LexState::BacktickQuoted => {
                escaped.push(ch);
                if ch == '`' {
                    state = LexState::Normal;
                }
            }
            LexState::LineComment => {
                escaped.push(ch);
                if ch == '\n' {
                    state = LexState::Normal;
                }
            }
            LexState::BlockComment => {
                escaped.push(ch);
                if ch == '*' && chars.peek() == Some(&'/') {
                    escaped.push(chars.next().expect("peeked block-comment delimiter"));
                    state = LexState::Normal;
                }
            }
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::{EXPR_CTX, escape_quoted_newlines, eval_condition};

    #[test]
    fn escapes_raw_newlines_in_interpreted_strings() {
        assert_eq!(
            escape_quoted_newlines("'line 1\nline 2' == \"line 1\r\nline 2\""),
            "'line 1\\nline 2' == \"line 1\\r\\nline 2\""
        );
    }

    #[test]
    fn leaves_multiline_strings_comments_and_whitespace_unchanged() {
        let code = "`raw\nstring` // 'comment\n\n&& true /* \"comment\n */";
        assert_eq!(escape_quoted_newlines(code), code);
    }

    #[test]
    fn evaluates_v1_style_raw_newline_literals() {
        assert_eq!(
            eval_condition("'ITWORKS\n' == 'ITWORKS\\n'", &EXPR_CTX).unwrap(),
            expr::Value::Bool(true)
        );
    }

    #[test]
    fn exec_returns_stdout() {
        assert_eq!(
            eval_condition("exec('printf hi')", &EXPR_CTX).unwrap(),
            expr::Value::String("hi".to_string())
        );
    }

    #[test]
    fn exec_reports_a_failing_command_as_an_error() {
        let error = eval_condition("exec('exit 3')", &EXPR_CTX).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("exited with code 3"), "{message}");
        assert!(message.contains("exec_ok()"), "{message}");
    }

    #[test]
    fn exec_rejects_missing_and_non_string_arguments_without_panicking() {
        for expression in ["exec()", "exec(1)", "exec('a', 'b')"] {
            let error = eval_condition(expression, &EXPR_CTX).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("exec() expects exactly one string argument"),
                "unexpected error for {expression}: {error}"
            );
        }
    }

    #[test]
    fn exec_rejects_output_that_is_not_utf8_without_panicking() {
        let error = eval_condition(r#"exec(`printf '\377'`)"#, &EXPR_CTX).unwrap_err();
        assert!(error.to_string().contains("not valid UTF-8"), "{error}");
    }

    #[test]
    fn exec_ok_reports_the_exit_status_as_a_bool() {
        assert_eq!(
            eval_condition("exec_ok('true')", &EXPR_CTX).unwrap(),
            expr::Value::Bool(true)
        );
        assert_eq!(
            eval_condition("exec_ok('exit 1')", &EXPR_CTX).unwrap(),
            expr::Value::Bool(false)
        );
        // Output, including bytes that are not UTF-8, is ignored.
        assert_eq!(
            eval_condition(r#"exec_ok(`printf '\377'; exit 2`)"#, &EXPR_CTX).unwrap(),
            expr::Value::Bool(false)
        );
        assert_eq!(
            eval_condition("!exec_ok('test -f definitely-missing-file')", &EXPR_CTX).unwrap(),
            expr::Value::Bool(true)
        );
    }

    #[test]
    fn exec_ok_rejects_missing_and_non_string_arguments() {
        for expression in ["exec_ok()", "exec_ok(1)"] {
            let error = eval_condition(expression, &EXPR_CTX).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("exec_ok() expects exactly one string argument"),
                "unexpected error for {expression}: {error}"
            );
        }
    }

    #[test]
    fn env_rejects_missing_and_non_string_arguments() {
        for expression in ["env()", "env(123)", "env('ONE', 'TWO')"] {
            let error = eval_condition(expression, &EXPR_CTX).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("env() expects exactly one string argument"),
                "unexpected error for {expression}: {error}"
            );
        }
    }
}
