use crate::{Result, progress_bar, style};
use serde::ser::Serialize;
use std::{
    collections::HashMap, fmt, sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering}, mpsc, Arc, LazyLock, Mutex, OnceLock, Weak
    }, thread, time::{Duration, Instant}
};

use console::Term;
use indicatif::TermLike;
use tera::{Context, Tera};

static DEFAULT_BODY: LazyLock<String> =
    LazyLock::new(|| "{{ spinner() }} {{ message }}".to_string());

struct Spinner {
    frames: Vec<String>,
    fps: usize,
}

macro_rules! spinner {
    ($name:expr, $frames:expr, $fps:expr) => {
        (
            $name.to_string(),
            Spinner {
                frames: $frames.iter().map(|s| s.to_string()).collect(),
                fps: $fps,
            },
        )
    };
}

const DEFAULT_SPINNER: &str = "mini_dot";
#[rustfmt::skip]
static SPINNERS: LazyLock<HashMap<String, Spinner>> = LazyLock::new(|| {
    vec![
        // from https://github.com/charmbracelet/bubbles/blob/ea344ab907bddf5e8f71cd73b9583b070e8f1b2f/spinner/spinner.go
        spinner!("line", &["|", "/", "-", "\\"], 200),
        spinner!("dot", &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"], 200),
        spinner!("mini_dot", &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"], 200),
        spinner!("jump", &["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"], 200),
        spinner!("pulse", &["█", "▓", "▒", "░"], 200),
        spinner!("points", &["∙∙∙", "●∙∙", "∙●∙", "∙∙●"], 200),
        spinner!("globe", &["🌍", "🌎", "🌏"], 400),
        spinner!("moon", &["🌑", "🌒", "🌓", "🌔", "🌕", "🌖", "🌗", "🌘"], 400),
        spinner!("monkey", &["🙈", "🙉", "🙊"], 400),
        spinner!("meter", &["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"], 400),
        spinner!("hamburger", &["☱", "☲", "☴", "☲"], 200),
        spinner!("ellipsis", &["   ", ".  ", ".. ", "..."], 200),
    ]
    .into_iter()
    .collect()
});

static INTERVAL: Mutex<Duration> = Mutex::new(Duration::from_millis(200)); // TODO: use fps from a spinner
static LINES: Mutex<usize> = Mutex::new(0);
static NOTIFY: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
static STARTED: Mutex<bool> = Mutex::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);
static JOBS: Mutex<Vec<Arc<ProgressJob>>> = Mutex::new(vec![]);
static TERA: Mutex<Option<Tera>> = Mutex::new(None);

#[derive(Clone)]
struct RenderContext {
    start: Instant,
    now: Instant,
    width: usize,
    tera_ctx: Context,
    indent: usize,
    include_children: bool,
    progress: Option<(usize, usize)>,
}

impl Default for RenderContext {
    fn default() -> Self {
        let mut tera_ctx = Context::new();
        tera_ctx.insert("message", "");
        Self {
            start: Instant::now(),
            now: Instant::now(),
            width: term().width() as usize,
            tera_ctx,
            indent: 0,
            include_children: true,
            progress: None,
        }
    }
}

impl RenderContext {
    pub fn elapsed(&self) -> Duration {
        self.now - self.start
    }
}

pub struct ProgressJobBuilder {
    body: String,
    body_text: Option<String>,
    status: ProgressStatus,
    ctx: Context,
    on_done: ProgressJobDoneBehavior,
    progress_current: Option<usize>,
    progress_total: Option<usize>,
}

impl Default for ProgressJobBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressJobBuilder {
    pub fn new() -> Self {
        Self {
            body: DEFAULT_BODY.clone(),
            body_text: None,
            status: Default::default(),
            ctx: Default::default(),
            on_done: Default::default(),
            progress_current: None,
            progress_total: None,
        }
    }

    pub fn body<S: Into<String>>(mut self, body: S) -> Self {
        self.body = body.into();
        self
    }

    pub fn body_text(mut self, body: Option<impl Into<String>>) -> Self {
        self.body_text = body.map(|s| s.into());
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

    pub fn progress_current(mut self, progress_current: usize) -> Self {
        self.progress_current = Some(progress_current);
        self
    }

    pub fn progress_total(mut self, progress_total: usize) -> Self {
        self.progress_total = Some(progress_total);
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
            body: Mutex::new(self.body),
            body_text: self.body_text,
            status: Mutex::new(self.status),
            on_done: self.on_done,
            parent: Weak::new(),
            children: Mutex::new(vec![]),
            tera_ctx: Mutex::new(self.ctx),
            progress_current: Mutex::new(self.progress_current),
            progress_total: Mutex::new(self.progress_total),
        }
    }

    pub fn start(self) -> Arc<ProgressJob> {
        let job = Arc::new(self.build());
        JOBS.lock().unwrap().push(job.clone());
        job.update();
        job
    }
}

#[derive(Debug, Default, Clone, PartialEq, strum::EnumIs)]
pub enum ProgressStatus {
    Hide,
    Pending,
    #[default]
    Running,
    RunningCustom(String),
    DoneCustom(String),
    Done,
    Warn,
    Failed,
}

impl ProgressStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::RunningCustom(_))
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum ProgressJobDoneBehavior {
    #[default]
    Keep,
    Collapse,
    Hide,
}

pub struct ProgressJob {
    id: usize,
    body: Mutex<String>,
    body_text: Option<String>,
    status: Mutex<ProgressStatus>,
    parent: Weak<ProgressJob>,
    children: Mutex<Vec<Arc<ProgressJob>>>,
    tera_ctx: Mutex<Context>,
    on_done: ProgressJobDoneBehavior,
    progress_current: Mutex<Option<usize>>,
    progress_total: Mutex<Option<usize>>,
}

impl ProgressJob {
    fn render(&self, tera: &mut Tera, mut ctx: RenderContext) -> Result<String> {
        let mut s = vec![];
        ctx.tera_ctx.extend(self.tera_ctx.lock().unwrap().clone());
        ctx.progress = if let (Some(progress_current), Some(progress_total)) = (
            *self.progress_current.lock().unwrap(),
            *self.progress_total.lock().unwrap(),
        ) {
            Some((progress_current, progress_total))
        } else {
            None
        };
        add_tera_functions(tera, &ctx, self);
        if !self.should_display() {
            return Ok(String::new());
        }
        let body = if output() == ProgressOutput::Text {
            self.body_text
                .clone()
                .unwrap_or(self.body.lock().unwrap().clone())
        } else {
            self.body.lock().unwrap().clone()
        };
        let name = format!("progress_{}", self.id);
        add_tera_template(tera, &name, &body)?;
        let body = tera.render(&name, &ctx.tera_ctx)?;
        let body = flex(&body, ctx.width - ctx.indent);
        s.push(body.trim_end().to_string());
        if ctx.include_children && self.should_display_children() {
            ctx.indent += 1;
            let children = self.children.lock().unwrap();
            for child in children.iter() {
                let child_output = child.render(tera, ctx.clone())?;
                if !child_output.is_empty() {
                    let child_output = indent(child_output, ctx.width - ctx.indent + 1, ctx.indent);
                    s.push(child_output);
                }
            }
        }
        Ok(s.join("\n"))
    }

    fn should_display(&self) -> bool {
        let status = self.status.lock().unwrap();
        !status.is_hide() && (status.is_active() || self.on_done != ProgressJobDoneBehavior::Hide)
    }

    fn should_display_children(&self) -> bool {
        self.status.lock().unwrap().is_active() || self.on_done == ProgressJobDoneBehavior::Keep
    }

    pub fn add(self: &Arc<Self>, mut job: ProgressJob) -> Arc<Self> {
        job.parent = Arc::downgrade(self);
        let job = Arc::new(job);
        self.children.lock().unwrap().push(job.clone());
        job.update();
        job
    }

    pub fn remove(&self) {
        if let Some(parent) = self.parent.upgrade() {
            parent
                .children
                .lock()
                .unwrap()
                .retain(|child| child.id != self.id);
        } else {
            JOBS.lock().unwrap().retain(|job| job.id != self.id);
        }
    }

    pub fn children(&self) -> Vec<Arc<Self>> {
        self.children.lock().unwrap().clone()
    }

    pub fn is_running(&self) -> bool {
        self.status.lock().unwrap().is_active()
    }

    pub fn set_body<S: Into<String>>(&self, body: S) {
        *self.body.lock().unwrap() = body.into();
        self.update();
    }

    pub fn set_status(&self, status: ProgressStatus) {
        let mut s = self.status.lock().unwrap();
        if *s != status {
            *s = status;
            drop(s);
            self.update();
        }
    }

    pub fn prop<T: Serialize + ?Sized, S: Into<String>>(&self, key: S, val: &T) {
        let mut ctx = self.tera_ctx.lock().unwrap();
        ctx.insert(key, val);
    }

    pub fn progress_current(&self, mut current: usize) {
        if let Some(total) = *self.progress_total.lock().unwrap() {
            current = current.min(total);
        }
        *self.progress_current.lock().unwrap() = Some(current);
        self.update();
    }

    pub fn progress_total(&self, mut total: usize) {
        if let Some(current) = *self.progress_current.lock().unwrap() {
            total = total.max(current);
        }
        *self.progress_total.lock().unwrap() = Some(total);
        self.update();
    }

    pub fn update(&self) {
        if output() == ProgressOutput::Text {
            let update = || {
                let mut ctx = RenderContext {
                    include_children: false,
                    ..Default::default()
                };
                ctx.tera_ctx.insert("message", "");
                let mut tera = TERA.lock().unwrap();
                if tera.is_none() {
                    *tera = Some(Tera::default());
                }
                let tera = tera.as_mut().unwrap();
                let output = self.render(tera, ctx)?;
                if !output.is_empty() {
                    term().write_line(&output)?;
                }
                Result::Ok(())
            };
            if let Err(e) = update() {
                eprintln!("clx: {e:?}");
            }
        } else {
            notify();
        }
    }

    pub fn println(&self, s: &str) {
        if !s.is_empty() {
            pause();
            let _ = term().write_line(s);
            resume();
        }
    }
}

impl fmt::Debug for ProgressJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProgressJob {{ id: {}, status: {:?} }}",
            self.id,
            self.status.lock().unwrap()
        )
    }
}

impl PartialEq for ProgressJob {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ProgressJob {}

fn indent(s: String, width: usize, indent: usize) -> String {
    let mut result = Vec::new();
    let indent_str = " ".repeat(indent);

    for line in s.lines() {
        let mut current = String::new();
        let mut current_width = 0;
        let mut chars = line.chars().peekable();
        let mut ansi_code = String::new();

        // Add initial indentation
        if current.is_empty() {
            current.push_str(&indent_str);
            current_width = indent;
        }

        while let Some(c) = chars.next() {
            // Handle ANSI escape codes
            if c == '\x1b' {
                ansi_code = String::from(c);
                while let Some(&next) = chars.peek() {
                    ansi_code.push(next);
                    chars.next();
                    if next == 'm' {
                        break;
                    }
                }
                current.push_str(&ansi_code);
                continue;
            }

            let char_width = console::measure_text_width(&c.to_string());
            let next_width = current_width + char_width;

            // Only wrap if we're not at the end of the input and the next character would exceed width
            if next_width > width && !current.trim().is_empty() && chars.peek().is_some() {
                result.push(current);
                current = format!("{}{}", indent_str, ansi_code);
                current_width = indent;
            }
            current.push(c);
            if !c.is_control() {
                current_width += char_width;
            }
        }

        // For the last line, if it's too long, we need to wrap it
        if !current.is_empty() {
            if current_width > width {
                let mut width_so_far = indent;
                let mut last_valid_pos = indent_str.len();
                let mut chars = current[indent_str.len()..].chars();

                while let Some(c) = chars.next() {
                    if !c.is_control() {
                        width_so_far += console::measure_text_width(&c.to_string());
                        if width_so_far > width {
                            break;
                        }
                    }
                    last_valid_pos = current.len() - chars.as_str().len() - 1;
                }

                let (first, second) = current.split_at(last_valid_pos + 1);
                result.push(first.to_string());
                current = format!("{}{}{}", indent_str, ansi_code, second);
            }
            result.push(current);
        }
    }

    result.join("\n")
}

fn notify() {
    start();
    if let Some(tx) = NOTIFY.lock().unwrap().clone() {
        let _ = tx.send(());
    }
}

fn notify_wait(timeout: Duration) -> bool {
    let (tx, rx) = mpsc::channel();
    NOTIFY.lock().unwrap().replace(tx);
    rx.recv_timeout(timeout).is_ok()
}

pub fn flush() {
    if !*STARTED.lock().unwrap() {
        return;
    }
    if let Err(err) = refresh() {
        eprintln!("clx: {err:?}");
    }
}

fn start() {
    let mut started = STARTED.lock().unwrap();
    if *started || output() == ProgressOutput::Text {
        return; // prevent multiple loops running at a time
    }
    thread::spawn(move || {
        let mut refresh_after = Instant::now();
        loop {
            if refresh_after > Instant::now() {
                thread::sleep(refresh_after - Instant::now());
            }
            refresh_after = Instant::now() + interval() / 2;
            match refresh() {
                Ok(true) => {}
                Ok(false) => {
                    break;
                }
                Err(err) => {
                    eprintln!("clx: {err:?}");
                    *LINES.lock().unwrap() = 0;
                }
            }
            notify_wait(interval());
        }
    });
    *started = true;
}

fn refresh() -> Result<bool> {
    if is_paused() {
        return Ok(true);
    }
    static RENDER_CTX: OnceLock<Mutex<RenderContext>> = OnceLock::new();
    let ctx = RENDER_CTX.get_or_init(|| Mutex::new(RenderContext::default()));
    ctx.lock().unwrap().now = Instant::now();
    let ctx = ctx.lock().unwrap().clone();
    let mut tera = TERA.lock().unwrap();
    if tera.is_none() {
        *tera = Some(Tera::default());
    }
    let tera = tera.as_mut().unwrap();
    let jobs = JOBS.lock().unwrap().clone();
    let any_running_check = || jobs.iter().any(|job| job.is_running());
    let any_running = any_running_check();
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
    term.clear_last_lines(*lines)?;
    if !output.is_empty() {
        term.write_line(&output)?;
    }
    *lines = output.split("\n").count();
    if !any_running && !any_running_check() {
        *STARTED.lock().unwrap() = false;
        return Ok(false); // stop looping if no active progress jobs are running before or after the refresh
    }
    Ok(true)
}

fn term() -> &'static Term {
    static TERM: LazyLock<Term> = LazyLock::new(Term::stderr);
    &TERM
}

pub fn interval() -> Duration {
    *INTERVAL.lock().unwrap()
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
    if output() == ProgressOutput::UI {
        notify();
    }
}

fn clear() -> Result<()> {
    let term = term();
    let mut lines = LINES.lock().unwrap();
    term.move_cursor_up(*lines)?;
    term.clear_to_end_of_screen()?;
    *lines = 0;
    Ok(())
}

fn add_tera_functions(tera: &mut Tera, ctx: &RenderContext, job: &ProgressJob) {
    let elapsed = ctx.elapsed().as_millis() as usize;
    let status = job.status.lock().unwrap().clone();
    let progress = ctx.progress;
    let width = ctx.width;
    tera.register_function(
        "spinner",
        move |props: &HashMap<String, tera::Value>| match status {
            ProgressStatus::Running if output() == ProgressOutput::Text => {
                Ok(" ".to_string().into())
            }
            ProgressStatus::Hide => Ok(" ".to_string().into()),
            ProgressStatus::Pending => Ok(style::eyellow("⏸").dim().to_string().into()),
            ProgressStatus::Running => {
                let name = props
                    .get("name")
                    .as_ref()
                    .and_then(|v| v.as_str())
                    .unwrap_or(DEFAULT_SPINNER);
                let spinner = SPINNERS.get(name).expect("spinner not found");
                let frame_index = (elapsed / spinner.fps) % spinner.frames.len();
                let frame = spinner.frames[frame_index].clone();
                Ok(style::eblue(frame).to_string().into())
            }
            ProgressStatus::Done => Ok(style::egreen("✔").bright().to_string().into()),
            ProgressStatus::Failed => Ok(style::ered("✗").to_string().into()),
            ProgressStatus::RunningCustom(ref s) => Ok(s.clone().into()),
            ProgressStatus::DoneCustom(ref s) => Ok(s.clone().into()),
            ProgressStatus::Warn => Ok(style::eyellow("⚠").to_string().into()),
        },
    );
    tera.register_function(
        "progress_bar",
        move |props: &HashMap<String, tera::Value>| {
            if let Some((progress_current, progress_total)) = progress {
                let width = props
                    .get("width")
                    .as_ref()
                    .and_then(|v| v.as_i64())
                    .map(|v| if v < 0 { width - (-v as usize) } else { v as usize })
                    .unwrap_or(width);
                let progress_bar =
                    progress_bar::progress_bar(progress_current, progress_total, width);
                Ok(progress_bar.into())
            } else {
                Ok("".to_string().into())
            }
        },
    );
    tera.register_filter(
        "flex",
        |value: &tera::Value, _: &HashMap<String, tera::Value>| {
            Ok(format!(
                "<clx:flex>{}<clx:flex>",
                value
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| value.to_string())
            )
            .into())
        },
    );
}

fn add_tera_template(tera: &mut Tera, name: &str, body: &str) -> Result<()> {
    if !tera.get_template_names().any(|n| n == name) {
        tera.add_raw_template(name, body)?;
    }
    Ok(())
}

#[derive(PartialEq, Clone, Copy)]
pub enum ProgressOutput {
    UI,
    Text,
}

static OUTPUT: Mutex<ProgressOutput> = Mutex::new(ProgressOutput::UI);

pub fn set_output(output: ProgressOutput) {
    *OUTPUT.lock().unwrap() = output;
}

pub fn output() -> ProgressOutput {
    *OUTPUT.lock().unwrap()
}

fn flex(s: &str, width: usize) -> String {
    let flex = |s: &str| {
        let mut result = String::new();
        let parts = s.splitn(3, "<clx:flex>").collect::<Vec<_>>();
        if parts.len() != 3 {
            return s.to_string();
        }
        let width =
            width - console::measure_text_width(parts[0]) - console::measure_text_width(parts[2]);
        result.push_str(parts[0]);
        // TODO: why +1?
        result.push_str(&console::truncate_str(parts[1], width, "…"));
        result.push_str(parts[2]);
        result
    };
    s.lines().map(flex).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indent() {
        let s = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let result = indent(s.to_string(), 10, 2);
        assert_eq!(
            result,
            "  aaaaaaaa\n  aaaaaaaa\n  aaaaaaaa\n  aaaaaaaa\n  aa"
        );

        let s = "\x1b[0;31maaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let result = indent(s.to_string(), 10, 2);
        assert_eq!(
            result,
            "  \x1b[0;31maaaaaaaa\n  \x1b[0;31maaaaaaaa\n  \x1b[0;31maaaaaaaa\n  \x1b[0;31maaaaaaaa\n  \x1b[0;31maa"
        );
    }
}
