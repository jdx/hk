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
pub static EXPR_ENV: LazyLock<expr::Environment> = LazyLock::new(|| {
    let mut env = expr::Environment::new();

    env.add_function("exec", |c| {
        let out = xx::process::sh(c.args[0].as_string().unwrap())
            .map_err(|e| expr::Error::ExprError(e.to_string()))?;
        Ok(expr::Value::String(out))
    });

    env
});
