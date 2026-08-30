//! Expression evaluation environment for step conditions.
//!
//! Provides the expression evaluation context used for `condition` fields
//! in step configurations. Supports custom functions like `exec()` for
//! running shell commands during condition evaluation.

use std::sync::LazyLock;

/// Default expression evaluation context.
pub static EXPR_CTX: LazyLock<expr::Context> = LazyLock::new(expr::Context::default);

/// Expression environment with custom functions.
///
/// Currently provides:
/// - `exec(command)` - Execute a shell command and return its stdout
/// - `env(name)` - Return an environment variable, or an empty string when unset
pub static EXPR_ENV: LazyLock<expr::Environment> = LazyLock::new(|| {
    let mut env = expr::Environment::new();

    env.add_function("exec", |c| {
        let out = xx::process::sh(c.args[0].as_string().unwrap())
            .map_err(|e| expr::Error::ExprError(e.to_string()))?;
        Ok(expr::Value::String(out))
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
