//! Process execution utilities
//!
//! This module provides convenient functions and builders for executing external processes
//! with better ergonomics than the standard library's `std::process` module.
//!
//! ## Features
//!
//! - Simple shell command execution with `sh()`
//! - Builder pattern for complex command construction
//! - Automatic stdout/stderr capture options
//! - Enhanced error messages that include the command that failed
//!
//! ## Examples
//!
//! ### Simple shell command
//!
//! ```rust,no_run
//! use xx::process;
//!
//! # fn main() -> xx::XXResult<()> {
//! // Run a shell command and get stdout as a string
//! let output = process::sh("echo hello world")?;
//! assert_eq!(output.trim(), "hello world");
//! # Ok(())
//! # }
//! ```
//!
//! ### Command builder
//!
//! ```rust,no_run
//! use xx::process;
//!
//! # fn main() -> xx::XXResult<()> {
//! // Build a command with arguments
//! let output = process::cmd("git", &["status", "--short"])
//!     .read()?;
//!
//! // Capture stdout and stderr separately
//! let result = process::cmd("make", &["test"])
//!     .stdout_capture()
//!     .stderr_capture()
//!     .run()?;
//! # Ok(())
//! # }
//! ```

use std::process::{Command, ExitStatus};
use std::{ffi::OsString, fmt, io, process::Output};

use duct::IntoExecutablePath;

use crate::{XXError, XXResult};

pub fn sh(script: &str) -> XXResult<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdin(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .map_err(|err| XXError::ProcessError(err, format!("sh -c {script}")))?;

    check_status(output.status)
        .map_err(|err| XXError::ProcessError(err, format!("sh -c {script}")))?;
    let stdout = String::from_utf8(output.stdout).expect("stdout is not utf-8");
    Ok(stdout)
}

pub fn check_status(status: ExitStatus) -> io::Result<()> {
    if status.success() {
        return Ok(());
    }
    let msg = if let Some(code) = status.code() {
        format!("exited with code {code}")
    } else {
        "terminated by signal".to_string()
    };
    Err(io::Error::other(msg))
}

#[derive(Debug, Default, Clone)]
pub struct XXExpression {
    program: OsString,
    args: Vec<OsString>,
    stdout_capture: bool,
    stderr_capture: bool,
}

pub fn cmd<T, U>(program: T, args: U) -> XXExpression
where
    T: IntoExecutablePath,
    U: IntoIterator,
    U::Item: Into<OsString>,
{
    let program = program.to_executable();
    let args = args.into_iter().map(|arg| arg.into()).collect::<Vec<_>>();
    XXExpression {
        program,
        args,
        ..Default::default()
    }
}

impl XXExpression {
    pub fn stdout_capture(mut self) -> Self {
        self.stdout_capture = true;
        self
    }

    pub fn stderr_capture(mut self) -> Self {
        self.stderr_capture = true;
        self
    }

    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.args.extend(args.into_iter().map(|arg| arg.into()));
        self
    }

    pub fn run(&self) -> XXResult<Output> {
        debug!("$ {self}");
        let expr = self.build_expr();
        expr.run()
            .map_err(|err| XXError::ProcessError(err, self.to_string()))
    }

    pub fn read(&self) -> XXResult<String> {
        debug!("$ {self}");
        let expr = self.build_expr();
        expr.read()
            .map_err(|err| XXError::ProcessError(err, self.to_string()))
    }

    fn build_expr(&self) -> duct::Expression {
        let mut expr = duct::cmd(self.program.clone(), self.args.clone());
        if self.stdout_capture {
            expr = expr.stdout_capture();
        }
        if self.stderr_capture {
            expr = expr.stderr_capture();
        }
        expr
    }
}

impl fmt::Display for XXExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.program.to_string_lossy(),
            self.args
                .iter()
                .map(|arg| arg.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_cmd() {
        let expr = cmd("echo", ["hello", "world"]).stdout_capture();
        let output = expr.run().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"hello world\n");
    }

    #[test]
    fn test_cmd_read() {
        let expr = cmd("echo", ["hello"]).arg("world").args(["foo", "bar"]);
        let output = expr.read().unwrap();
        assert_eq!(output, "hello world foo bar");
    }
}
