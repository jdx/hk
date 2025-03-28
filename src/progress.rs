use crate::Result;
use serde::ser::Serialize;
use std::{
    sync::{
        Arc, LazyLock, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use console::Term;
use indicatif::TermLike;
use tera::{Context, Tera};

const DEFAULT_BODY: &str = "{{ spinner }} {{ name }}\n{{ body }}";
const SPINNER: &str = "⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈";

static INTERVAL: Mutex<Duration> = Mutex::new(Duration::from_millis(100));
static LINES: Mutex<usize> = Mutex::new(0);
static NOTIFY: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
static PAUSED: AtomicBool = AtomicBool::new(false);
static JOBS: Mutex<Vec<Arc<ProgressJob>>> = Mutex::new(vec![]);

#[derive(Clone, Default)]
struct RenderContext {
    width: usize,
    tera_ctx: Context,
    indent: usize,
}

pub struct ProgressBuilder {
    name: String,
    body: String,
    status: ProgressStatus,
    ctx: Context,
}

impl ProgressBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            body: DEFAULT_BODY.to_string(),
            status: Default::default(),
            ctx: Default::default(),
        }
    }

    pub fn body(mut self, body: String) -> Self {
        self.body = body;
        self
    }

    pub fn status(mut self, status: ProgressStatus) -> Self {
        self.status = status;
        self
    }

    pub fn prop<T: Serialize + ?Sized, S: Into<String>>(mut self, key: S, val: &T) -> Self {
        self.ctx.insert(key, val);
        self
    }

    fn build_(self) -> ProgressJob {
        ProgressJob {
            name: self.name,
            body: Mutex::new(self.body),
            status: Mutex::new(self.status),
            parent: Weak::new(),
            children: Mutex::new(vec![]),
            tera_ctx: Mutex::new(self.ctx),
        }
    }

    pub fn build(self) -> Arc<ProgressJob> {
        let job = Arc::new(self.build_());
        JOBS.lock().unwrap().push(job.clone());
        start();
        job
    }
}

#[derive(Default, PartialEq)]
pub enum ProgressStatus {
    Pending,
    #[default]
    Running,
    Done,
    Failed,
    Custom(String),
}

pub struct ProgressJob {
    // id: String,
    name: String,
    body: Mutex<String>,
    status: Mutex<ProgressStatus>,
    parent: Weak<ProgressJob>,
    children: Mutex<Vec<Arc<ProgressJob>>>,
    tera_ctx: Mutex<Context>,
}

impl ProgressJob {
    fn render(&self, tera: &mut Tera, ctx: &RenderContext) -> Result<String> {
        let mut s = vec![];
        let mut ctx = ctx.clone();
        ctx.tera_ctx.extend(self.tera_ctx.lock().unwrap().clone());
        ctx.tera_ctx.insert("name", &self.name);
        match *self.status.lock().unwrap() {
            ProgressStatus::Pending => {
                return Ok(String::new());
            }
            ProgressStatus::Running => {
                // ctx.tera_ctx.insert("spinner", &spinner());
            }
            ProgressStatus::Done => {
                ctx.tera_ctx.insert("spinner", &"✔");
            }
            ProgressStatus::Failed => {
                ctx.tera_ctx.insert("spinner", &"✗");
            }
            ProgressStatus::Custom(ref s) => {
                ctx.tera_ctx.insert("spinner", &s);
            }
        }
        let body = tera.render_str(&self.body(), &ctx.tera_ctx)?;
        s.push(body.trim_end().to_string());
        ctx.indent += 2;
        let children = self.children.lock().unwrap();
        for child in children.iter() {
            let child_output = child.render(tera, &ctx)?;
            if !child_output.is_empty() {
                let child_output = indent(child_output, ctx.width, ctx.indent);
                s.push(child_output);
            }
        }
        Ok(s.join("\n"))
    }

    pub fn add(self: &Arc<Self>, pb: ProgressBuilder) -> Arc<Self> {
        let mut job = pb.build_();
        job.parent = Arc::downgrade(&self);
        let job = Arc::new(job);
        self.children.lock().unwrap().push(job.clone());
        start();
        job
    }

    pub fn set_body(&self, body: String) {
        *self.body.lock().unwrap() = body;
    }

    pub fn body(&self) -> String {
        self.body.lock().unwrap().clone()
    }

    pub fn is_running(&self) -> bool {
        *self.status.lock().unwrap() == ProgressStatus::Running
    }

    pub fn set_status(&self, status: ProgressStatus) {
        *self.status.lock().unwrap() = status;
        notify();
    }

    pub fn add_prop<T: Serialize + ?Sized, S: Into<String>>(&self, key: S, val: &T) {
        let mut ctx = self.tera_ctx.lock().unwrap();
        ctx.insert(key, val);
    }
}

fn indent(s: String, width: usize, indent: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut result = Vec::new();
    let indent_str = " ".repeat(indent);

    for line in s.lines() {
        let mut current = String::new();
        let mut current_width = 0;

        // Add initial indentation
        if current.is_empty() {
            current.push_str(&indent_str);
            current_width = indent;
        }

        for c in line.chars() {
            let char_width = c.width().unwrap_or(1);
            if current_width + char_width > width && !current.trim().is_empty() {
                result.push(current);
                current = indent_str.clone();
                current_width = indent;
            }
            current.push(c);
            if !c.is_control() {
                current_width += char_width;
            }
        }

        if !current.is_empty() {
            result.push(current);
        }
    }

    result.join("\n")
}

fn notify() {
    if let Some(tx) = NOTIFY.lock().unwrap().clone() {
        let _ = tx.send(());
    }
}

fn notify_wait(timeout: Duration) -> bool {
    let (tx, rx) = mpsc::channel();
    NOTIFY.lock().unwrap().replace(tx);
    rx.recv_timeout(timeout).is_ok()
}

fn start() {
    static STARTED: Mutex<bool> = Mutex::new(false);
    let mut started = STARTED.lock().unwrap();
    if *started {
        return;
    }
    thread::spawn(move || {
        let mut tera = Tera::default();
        let mut ctx = RenderContext::default();
        ctx.tera_ctx.insert("body", "");
        ctx.tera_ctx.insert("spinner", &spinner());
        loop {
            ctx.width = term().width() as usize;
            let jobs = JOBS.lock().unwrap().clone();
            if let Err(err) = refresh(&jobs, &mut tera, ctx.clone()) {
                eprintln!("clx: {:?}", err);
                *LINES.lock().unwrap() = 0;
            }
            if !jobs.iter().any(|job| job.is_running()) {
                *STARTED.lock().unwrap() = false;
                return;
            }
            if !notify_wait(interval()) {
                // only update spinner if timed out
                ctx.tera_ctx.insert("spinner", &spinner());
            }
        }
    });
    *started = true;
}

fn refresh(jobs: &[Arc<ProgressJob>], tera: &mut Tera, ctx: RenderContext) -> Result<()> {
    if is_paused() {
        return Ok(());
    }
    let term = term();
    let mut lines = LINES.lock().unwrap();
    let output = jobs
        .iter()
        .map(|job| job.render(tera, &ctx))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    term.move_cursor_up(*lines)?;
    term.clear_to_end_of_screen()?;
    term.write_line(&output)?;
    *lines = output.split("\n").fold(0, |acc, line| {
        acc + 1 + console::measure_text_width(line) / ctx.width
    });
    Ok(())
}

fn term() -> &'static Term {
    static TERM: LazyLock<Term> = LazyLock::new(|| Term::stderr());
    &TERM
}

pub fn interval() -> Duration {
    INTERVAL.lock().unwrap().clone()
}

pub fn set_interval(interval: Duration) {
    *INTERVAL.lock().unwrap() = interval;
}

pub fn is_paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

pub fn pause() {
    PAUSED.store(true, Ordering::Relaxed);
    let _ = clear();
}

pub fn resume() {
    PAUSED.store(false, Ordering::Relaxed);
    notify();
}

fn clear() -> Result<()> {
    let term = term();
    let mut lines = LINES.lock().unwrap();
    term.move_cursor_up(*lines)?;
    term.clear_to_end_of_screen()?;
    *lines = 0;
    Ok(())
}

fn spinner() -> char {
    static INC: AtomicUsize = AtomicUsize::new(0);
    let inc = INC.fetch_add(1, Ordering::Relaxed);
    SPINNER.chars().nth(inc % SPINNER.len()).unwrap()
}
