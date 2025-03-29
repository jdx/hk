use crate::Result;
use serde::ser::Serialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, LazyLock, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use console::Term;
use indicatif::TermLike;
use tera::{Context, Tera};

const DEFAULT_BODY: LazyLock<Vec<String>> =
    LazyLock::new(|| vec!["{{ spinner() }} {{ message }}".to_string()]);

struct Spinner {
    frames: Vec<String>,
    fps: Duration,
}

macro_rules! spinner {
    ($name:ident, $frames:expr, $fps:expr) => {
        Spinner {
            frames: $frames.iter().map(|s| s.to_string()).collect(),
            fps: Duration::from_millis($fps),
        }
    };
}

#[rustfmt::skip]
static SPINNERS: LazyLock<HashMap<String, Spinner>> = LazyLock::new(|| {
    vec![
        // from https://github.com/charmbracelet/bubbles/blob/ea344ab907bddf5e8f71cd73b9583b070e8f1b2f/spinner/spinner.go
        ("line".to_string(), spinner!(line, &["|", "/", "-", "\\"], 100)),
        ("dot".to_string(), spinner!(dot, &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"], 100)),
        ("mini_dot".to_string(), spinner!(mini_dot, &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"], 80)),
        ("jump".to_string(), spinner!(jump, &["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"], 100)),
        ("pulse".to_string(), spinner!(pulse, &["█", "▓", "▒", "░"], 120)),
        ("points".to_string(), spinner!(points, &["∙∙∙", "●∙∙", "∙●∙", "∙∙●"], 150)),
        ("globe".to_string(), spinner!(globe, &["🌍", "🌎", "🌏"], 250)),
        ("moon".to_string(), spinner!(moon, &["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"], 120)),
        ("monkey".to_string(), spinner!(monkey, &["🙈", "🙉", "🙊"], 300)),
        ("meter".to_string(), spinner!(meter, &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"], 120)),
        ("hamburger".to_string(), spinner!(hamburger, &["☱", "☲", "☴", "☲"], 120)),
        ("ellipsis".to_string(), spinner!(ellipsis, &["", ".", "..", "..."], 120)),
    ]
    .into_iter()
    .collect()
});

static INTERVAL: Mutex<Duration> = Mutex::new(Duration::from_millis(100)); // TODO: use fps from a spinner
static LINES: Mutex<usize> = Mutex::new(0);
static NOTIFY: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
static PAUSED: AtomicBool = AtomicBool::new(false);
static JOBS: Mutex<Vec<Arc<ProgressJob>>> = Mutex::new(vec![]);
static TERA: LazyLock<Tera> = LazyLock::new(tera);

#[derive(Clone)]
struct RenderContext {
    start: Instant,
    now: Instant,
    width: usize,
    tera_ctx: Context,
    indent: usize,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            now: Instant::now(),
            width: 0,
            tera_ctx: Context::new(),
            indent: 0,
        }
    }
}

impl RenderContext {
    pub fn elapsed(&self) -> Duration {
        self.now - self.start
    }
}

pub struct ProgressJobBuilder {
    pub body: Vec<String>,
    status: ProgressStatus,
    ctx: Context,
    on_done: ProgressJobDoneBehavior,
}

impl ProgressJobBuilder {
    pub fn new() -> Self {
        Self {
            body: DEFAULT_BODY.clone(),
            status: Default::default(),
            ctx: Default::default(),
            on_done: Default::default(),
        }
    }

    pub fn body(mut self, body: Vec<String>) -> Self {
        self.body = body;
        self
    }

    pub fn status(mut self, status: ProgressStatus) -> Self {
        self.status = status;
        self
    }

    pub fn on_done(mut self, on_done: ProgressJobDoneBehavior) -> Self {
        self.on_done = on_done;
        self
    }

    pub fn prop<T: Serialize + ?Sized, S: Into<String>>(mut self, key: S, val: &T) -> Self {
        self.ctx.insert(key, val);
        self
    }

    pub fn build(self) -> ProgressJob {
        static ID: AtomicUsize = AtomicUsize::new(0);
        ProgressJob {
            id: ID.fetch_add(1, Ordering::Relaxed),
            body: self.body,
            status: Mutex::new(self.status),
            on_done: self.on_done,
            parent: Weak::new(),
            children: Mutex::new(vec![]),
            tera_ctx: Mutex::new(self.ctx),
        }
    }

    pub fn start(self) -> Arc<ProgressJob> {
        let job = Arc::new(self.build());
        JOBS.lock().unwrap().push(job.clone());
        start();
        job
    }
}

#[derive(Default, Clone, PartialEq, strum::EnumIs)]
pub enum ProgressStatus {
    Pending,
    #[default]
    Running,
    Custom(String),
    Done,
    Failed,
}

impl ProgressStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Custom(_))
    }
}

#[derive(Default, PartialEq)]
pub enum ProgressJobDoneBehavior {
    #[default]
    Keep,
    Collapse,
    Hide,
}

pub struct ProgressJob {
    id: usize,
    body: Vec<String>,
    status: Mutex<ProgressStatus>,
    parent: Weak<ProgressJob>,
    children: Mutex<Vec<Arc<ProgressJob>>>,
    tera_ctx: Mutex<Context>,
    on_done: ProgressJobDoneBehavior,
}

impl ProgressJob {
    fn render(self: &Arc<Self>, tera: &mut Tera, mut ctx: RenderContext) -> Result<String> {
        let mut s = vec![];
        ctx.tera_ctx.extend(self.tera_ctx.lock().unwrap().clone());
        add_tera_functions(tera, &mut ctx, self.clone());
        if !self.should_display() {
            return Ok(String::new());
        }
        for (body_id, body) in self.body.iter().enumerate() {
            let name = format!("progress_{}_{}", self.id, body_id);
            add_tera_template(tera, &name, &body)?;
            let body = tera.render(&name, &ctx.tera_ctx)?;
            s.push(body.trim_end().to_string());
        }
        if self.should_display_children() {
            ctx.indent += 1;
            let children = self.children.lock().unwrap();
            for child in children.iter() {
                let child_output = child.render(tera, ctx.clone())?;
                if !child_output.is_empty() {
                    let child_output = indent(child_output, ctx.width, ctx.indent);
                    s.push(child_output);
                }
            }
        }
        Ok(s.join("\n"))
    }

    fn should_display(&self) -> bool {
        let status = self.status.lock().unwrap();
        !status.is_pending() && (status.is_active() || self.on_done != ProgressJobDoneBehavior::Hide)
    }

    fn should_display_children(&self) -> bool {
        self.status.lock().unwrap().is_active() || self.on_done == ProgressJobDoneBehavior::Keep
    }

    pub fn add(self: &Arc<Self>, mut job: ProgressJob) -> Arc<Self> {
        job.parent = Arc::downgrade(&self);
        let job = Arc::new(job);
        self.children.lock().unwrap().push(job.clone());
        start();
        job
    }

    pub fn is_running(&self) -> bool {
        self.status.lock().unwrap().is_active()
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
        return; // prevent multiple loops running at a time
    }
    thread::spawn(move || {
        let mut tera = TERA.clone();
        let mut ctx = RenderContext::default();
        ctx.tera_ctx.insert("message", "");
        loop {
            ctx.now = Instant::now();
            ctx.width = term().width() as usize;
            let jobs = JOBS.lock().unwrap().clone();
            if let Err(err) = refresh(&jobs, &mut tera, ctx.clone()) {
                eprintln!("clx: {:?}", err);
                *LINES.lock().unwrap() = 0;
            }
            if !jobs.iter().any(|job| job.is_running()) {
                *STARTED.lock().unwrap() = false;
                return; // stop looping if no active progress jobs are running
            }
            notify_wait(interval());
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
        .map(|job| job.render(tera, ctx.clone()))
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

fn get_all_jobs(jobs: &[Arc<ProgressJob>]) -> Vec<Arc<ProgressJob>> {
    let mut all_jobs = jobs.to_vec();
    for job in jobs {
        let children = job.children.lock().unwrap().clone();
        all_jobs.extend(get_all_jobs(&children));
    }
    all_jobs
}

fn tera() -> Tera {
    Tera::default()
}

fn add_tera_functions(tera: &mut Tera, ctx: &RenderContext, job: Arc<ProgressJob>) {
    let elapsed = ctx.elapsed().as_millis() as usize;
    tera.register_function(
        "spinner",
        move |props: &HashMap<String, tera::Value>| match *job.status.lock().unwrap() {
            ProgressStatus::Running => {
                let name = props
                    .get("name")
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or("dot");
                let frame = SPINNERS.get(name).unwrap().frames[elapsed % SPINNERS.get(name).unwrap().frames.len()].clone();
                Ok(console::style(frame).blue().to_string().into())
            }
            ProgressStatus::Pending => Ok(" ".to_string().into()),
            ProgressStatus::Done => Ok(console::style("✔").bright().green().to_string().into()),
            ProgressStatus::Failed => Ok(console::style("✗").red().to_string().into()),
            ProgressStatus::Custom(ref s) => Ok(s.clone().into()),
        },
    );
}

fn add_tera_template(tera: &mut Tera, name: &str, body: &str) -> Result<()> {
    if !tera.get_template_names().any(|n| n == name) {
        tera.add_raw_template(name, body)?;
    }
    Ok(())
}
