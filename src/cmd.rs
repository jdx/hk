use crate::Result;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;
use tokio::{io::BufReader, process::Command, sync::Mutex};
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use indexmap::IndexSet;
use std::sync::LazyLock as Lazy;

use crate::Error::ScriptFailed;
use clx::ProgressJob;

pub struct CmdLineRunner {
    cmd: Command,
    program: String,
    args: Vec<String>,
    pr: Option<Arc<ProgressJob>>,
    stdin: Option<String>,
    redactions: IndexSet<String>,
    pass_signals: bool,
}

static RUNNING_PIDS: Lazy<std::sync::Mutex<HashSet<u32>>> = Lazy::new(Default::default);

impl CmdLineRunner {
    pub fn new<P: AsRef<OsStr>>(program: P) -> Self {
        let program = program.as_ref().to_string_lossy().to_string();
        let mut cmd = if cfg!(windows) {
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/c").arg(&program);
            cmd
        } else {
            Command::new(&program)
        };
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        Self {
            cmd,
            program,
            args: vec![],
            pr: None,
            stdin: None,
            redactions: Default::default(),
            pass_signals: false,
        }
    }

    #[cfg(unix)]
    pub fn kill_all(signal: nix::sys::signal::Signal) {
        let pids = RUNNING_PIDS.lock().unwrap();
        for pid in pids.iter() {
            let pid = *pid as i32;
            trace!("{signal}: {pid}");
            if let Err(e) = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), signal) {
                debug!("Failed to kill cmd {pid}: {e}");
            }
        }
    }

    #[cfg(windows)]
    pub fn kill_all() {
        let pids = RUNNING_PIDS.lock().unwrap();
        for pid in pids.iter() {
            if let Err(e) = Command::new("taskkill")
                .arg("/F")
                .arg("/T")
                .arg("/PID")
                .arg(pid.to_string())
                .spawn()
            {
                warn!("Failed to kill cmd {pid}: {e}");
            }
        }
    }

    pub fn stdin<T: Into<Stdio>>(mut self, cfg: T) -> Self {
        self.cmd.stdin(cfg);
        self
    }

    pub fn stdout<T: Into<Stdio>>(mut self, cfg: T) -> Self {
        self.cmd.stdout(cfg);
        self
    }

    pub fn stderr<T: Into<Stdio>>(mut self, cfg: T) -> Self {
        self.cmd.stderr(cfg);
        self
    }

    pub fn redact(mut self, redactions: impl IntoIterator<Item = String>) -> Self {
        for r in redactions {
            self.redactions.insert(r);
        }
        self
    }

    pub fn with_pr(mut self, pr: Arc<ProgressJob>) -> Self {
        self.pr = Some(pr);
        self
    }

    pub fn current_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.cmd.current_dir(dir);
        self
    }

    pub fn env_clear(mut self) -> Self {
        self.cmd.env_clear();
        self
    }

    pub fn env<K, V>(mut self, key: K, val: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.cmd.env(key, val);
        self
    }
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.cmd.envs(vars);
        self
    }

    pub fn opt_arg<S: AsRef<OsStr>>(mut self, arg: Option<S>) -> Self {
        if let Some(arg) = arg {
            self.cmd.arg(arg);
        }
        self
    }

    pub fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.cmd.arg(arg.as_ref());
        self.args.push(arg.as_ref().to_string_lossy().to_string());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args = args.into_iter().map(|s| s.as_ref().to_string_lossy().to_string()).collect::<Vec<_>>();
        self.cmd.args(&args);
        self.args.extend(args);
        self
    }

    pub fn with_pass_signals(&mut self) -> &mut Self {
        self.pass_signals = true;
        self
    }

    pub fn stdin_string(mut self, input: impl Into<String>) -> Self {
        self.cmd.stdin(Stdio::piped());
        self.stdin = Some(input.into());
        self
    }

    pub async fn execute(mut self) -> Result<CmdResult> {
        debug!("$ {self}");
        let mut cp = self.cmd.spawn()?;
        let id = cp.id().unwrap();
        RUNNING_PIDS.lock().unwrap().insert(id);
        trace!("Started process: {id} for {}", self.program);
        let result = Arc::new(Mutex::new(CmdResult::default()));
        let combined_output = Arc::new(Mutex::new(Vec::new()));
        if let Some(stdout) = cp.stdout.take() {
            let result = result.clone();
            let combined_output = combined_output.clone();
            let redactions = self.redactions.clone();
            let pr = self.pr.clone();
            tokio::spawn(async move {
                let stdout = BufReader::new(stdout);
                let mut lines = stdout.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let line = redactions
                        .iter()
                        .fold(line, |acc, r| acc.replace(r, "[redacted]"));
                    let mut result = result.lock().await;
                    result.stdout += &line;
                    result.stdout += "\n";
                    if let Some(pr) = &pr {
                        pr.add_prop("message", &line);
                    }
                    combined_output.lock().await.push(line);
                }
            });
        }
        if let Some(stderr) = cp.stderr.take() {
            let result = result.clone();
            let combined_output = combined_output.clone();
            let redactions = self.redactions.clone();
            let pr = self.pr.clone();
            tokio::spawn(async move {
                let stderr = BufReader::new(stderr);
                let mut lines = stderr.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let line = 
                        redactions
                        .iter()
                        .fold(line, |acc, r| acc.replace(r, "[redacted]"));
                        let mut result = result.lock().await;
                    result.stderr += &line;
                    result.stderr += "\n";
                    if pr.is_some() {
                        ProgressJob::pause();
                        eprintln!("{}", &line);
                        ProgressJob::resume();
                    }
                    combined_output.lock().await.push(line);
                }
            });
        }
        if let Some(text) = self.stdin.take() {
            let mut stdin = cp.stdin.take().unwrap();
            tokio::spawn(async move {
                stdin.write_all(text.as_bytes()).await.unwrap();
            });
        }
        let status = cp.wait().await.unwrap();
        RUNNING_PIDS.lock().unwrap().remove(&id);
        result.lock().await.status = status;

        if !status.success() {
            let result = result.lock().await.to_owned();
            self.on_error(combined_output.lock().await.join("\n"), result)?;
        }

        let result = result.lock().await.to_owned();
        Ok(result)
    }

    fn on_error(&self, output: String, result: CmdResult) -> Result<()> {
        let output = output.trim().to_string();
        // if let Some(pr) = &self.pr {
        //     if !output.trim().is_empty() {
        //         pr.println(output);
        //     }
        // }
        Err(ScriptFailed(self.program.clone(), output, result))?
    }
}

impl Display for CmdLineRunner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let args = self.args.join(" ");
        let mut cmd = format!("{} {}", &self.program, args);
        if cmd.starts_with("sh -c ") {
            cmd = cmd[6..].to_string();
        }
        write!(f, "{}", cmd)
    }
}

impl Debug for CmdLineRunner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let args = self.args.join(" ");
        write!(f, "{} {args}", self.program)
    }
}

#[derive(Debug, Default, Clone)]
pub struct CmdResult {
    pub stdout: String,
    pub stderr: String,
    pub status: ExitStatus,
}
