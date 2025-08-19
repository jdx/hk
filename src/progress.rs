use crate::{Result, progress_bar, style};
use serde::ser::Serialize;
use std::{
    collections::HashMap,
    fmt,
    sync::{
        Arc, LazyLock, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use console::Term;
use indicatif::TermLike;
use tera::{Context, Tera};
use tracing::{debug, trace};

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
static TERM_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Execute the provided function while holding the global terminal lock.
/// This allows external crates to synchronize stderr writes (e.g., logging)
/// with clx's progress clear/write operations to avoid interleaved output.
pub fn with_terminal_lock<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = TERM_LOCK.lock().unwrap();
    let result = f();
    drop(_guard);
    result
}
static REFRESH_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static STOPPING: AtomicBool = AtomicBool::new(false);
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
        crate::init();
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
        let rendered_body = tera.render(&name, &ctx.tera_ctx)?;
        trace!(
            template_name = %name,
            rendered_len = rendered_body.len(),
            width = ctx.width,
            indent = ctx.indent,
            "progress: rendered template"
        );
        if rendered_body.len() > 100 {
            trace!(preview = ?&rendered_body[..100], "progress: rendered preview");
        }
        let flex_width = ctx.width.saturating_sub(ctx.indent);
        let body = flex(&rendered_body, flex_width);
        trace!(
            flexed_len = body.len(),
            flex_width = flex_width,
            "progress: after flex"
        );
        // Safety check: if flex tags still exist, log a warning
        if body.contains("<clx:flex>") {
            debug!(
                job_id = self.id,
                body_preview = ?&body[..body.len().min(200)],
                "progress: flex tags remain after processing!"
            );
        }
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
        if STOPPING.load(Ordering::Relaxed) {
            return;
        }
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
                    // Safety check: ensure no flex tags are visible
                    let final_output = if output.contains("<clx:flex>") {
                        flex(&output, term().width() as usize)
                    } else {
                        output
                    };
                    let _guard = TERM_LOCK.lock().unwrap();
                    term().write_line(&final_output)?;
                    drop(_guard);
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
            // Safety check: ensure no flex tags are visible
            let output = if s.contains("<clx:flex>") {
                flex(s, term().width() as usize)
            } else {
                s.to_string()
            };
            let _guard = TERM_LOCK.lock().unwrap();
            let _ = term().write_line(&output);
            drop(_guard);
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
    if STOPPING.load(Ordering::Relaxed) {
        return;
    }
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
    if *started || output() == ProgressOutput::Text || STOPPING.load(Ordering::Relaxed) {
        return; // prevent multiple loops running at a time
    }
    // Mark as started BEFORE spawning to avoid a race that can start two loops
    *started = true;
    drop(started);
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
}

fn refresh() -> Result<bool> {
    let _refresh_guard = REFRESH_LOCK.lock().unwrap();
    if STOPPING.load(Ordering::Relaxed) {
        *STARTED.lock().unwrap() = false;
        return Ok(false);
    }
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
    // Perform clear + write + line accounting atomically to avoid interleaving with logger/pause
    let _guard = TERM_LOCK.lock().unwrap();
    // Robustly clear the previously rendered frame. Using move_cursor_up + clear_to_end_of_screen
    // avoids issues with terminals that wrap long lines differently.
    if *lines > 0 {
        trace!(prev_lines = *lines, "progress: clearing previous frame");
        // Clear wrapped rows explicitly to handle terminal wrapping correctly
        term.move_cursor_up(*lines)?;
        term.move_cursor_left(term.width() as usize)?;
        term.clear_to_end_of_screen()?;
    }
    if !output.is_empty() {
        // Safety check: ensure no flex tags are visible in final output
        let final_output = if output.contains("<clx:flex>") {
            // Process any remaining flex tags with terminal width
            flex(&output, term.width() as usize)
        } else {
            output
        };
        if final_output.contains("<clx:flex>") {
            trace!(
                final_output = final_output,
                "progress: flex tags should not be visible in final output"
            );
        }
        // Log a brief frame summary for diagnostics
        let newlines = final_output.lines().count();
        let first_line = final_output.lines().next().unwrap_or("");
        trace!(lines=newlines, chars=final_output.len(), first_line=?first_line, "progress: frame summary");
        term.write_line(&final_output)?;
        // Count how many terminal rows were actually consumed, accounting for wrapping
        let term_width = term.width() as usize;
        let mut consumed_rows = 0usize;
        for line in final_output.lines() {
            // Measure visible width (ANSI-safe)
            let visible_width = console::measure_text_width(line).max(1);
            // Number of rows this line occupies when wrapped on the terminal
            let rows = if term_width == 0 {
                1
            } else {
                (visible_width - 1) / term_width + 1
            };
            consumed_rows += rows.max(1);
        }
        trace!(
            consumed_rows = consumed_rows,
            term_width = term_width,
            "progress: computed consumed rows"
        );
        *lines = consumed_rows.max(1);
        trace!(stored_lines = *lines, "progress: after write state");
    } else {
        *lines = 0;
    }
    drop(_guard);
    if !any_running && !any_running_check() {
        *STARTED.lock().unwrap() = false;
        return Ok(false); // stop looping if no active progress jobs are running before or after the refresh
    }
    Ok(true)
}

fn refresh_once() -> Result<()> {
    let _refresh_guard = REFRESH_LOCK.lock().unwrap();
    let mut tera = TERA.lock().unwrap();
    if tera.is_none() {
        *tera = Some(Tera::default());
    }
    let tera = tera.as_mut().unwrap();
    static RENDER_CTX: OnceLock<Mutex<RenderContext>> = OnceLock::new();
    let ctx = RENDER_CTX.get_or_init(|| Mutex::new(RenderContext::default()));
    ctx.lock().unwrap().now = Instant::now();
    let ctx = ctx.lock().unwrap().clone();
    let jobs = JOBS.lock().unwrap().clone();
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
    let _guard = TERM_LOCK.lock().unwrap();
    if *lines > 0 {
        term.move_cursor_up(*lines)?;
        term.move_cursor_left(term.width() as usize)?;
        term.clear_to_end_of_screen()?;
    }
    if !output.is_empty() {
        let final_output = if output.contains("<clx:flex>") {
            flex(&output, term.width() as usize)
        } else {
            output
        };
        if final_output.contains("<clx:flex>") {
            trace!(
                final_output = final_output,
                "progress: flex tags should not be visible in final output"
            );
        }
        term.write_line(&final_output)?;
        let term_width = term.width() as usize;
        let mut consumed_rows = 0usize;
        for line in final_output.lines() {
            let visible_width = console::measure_text_width(line).max(1);
            let rows = if term_width == 0 {
                1
            } else {
                (visible_width - 1) / term_width + 1
            };
            consumed_rows += rows.max(1);
        }
        *lines = consumed_rows.max(1);
    } else {
        *lines = 0;
    }
    drop(_guard);
    Ok(())
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
    if *STARTED.lock().unwrap() {
        let _ = clear();
    }
}

pub fn resume() {
    PAUSED.store(false, Ordering::Relaxed);
    if !*STARTED.lock().unwrap() {
        return;
    }
    if output() == ProgressOutput::UI {
        notify();
    }
}

pub fn stop() {
    // Stop the refresh loop and finalize a last frame synchronously
    STOPPING.store(true, Ordering::Relaxed);
    let _ = refresh_once();
    *STARTED.lock().unwrap() = false;
}

pub fn stop_clear() {
    // Stop immediately and clear any progress from the screen
    STOPPING.store(true, Ordering::Relaxed);
    let _ = clear();
    *STARTED.lock().unwrap() = false;
}

fn clear() -> Result<()> {
    let term = term();
    let mut lines = LINES.lock().unwrap();
    if *lines > 0 {
        let _guard = TERM_LOCK.lock().unwrap();
        term.move_cursor_up(*lines)?;
        term.move_cursor_left(term.width() as usize)?;
        term.clear_to_end_of_screen()?;
        drop(_guard);
    }
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
                    .map(|v| {
                        if v < 0 {
                            width - (-v as usize)
                        } else {
                            v as usize
                        }
                    })
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
            let content = value
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| value.to_string());
            Ok(format!("<clx:flex>{}<clx:flex>", content).into())
        },
    );

    // Simple truncate filter for text mode
    tera.register_filter(
        "truncate_text",
        move |value: &tera::Value, args: &HashMap<String, tera::Value>| {
            let content = value
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| value.to_string());

            let prefix_len = args
                .get("prefix_len")
                .and_then(|v| v.as_i64())
                .map(|v| v as usize)
                .unwrap_or(20); // Default prefix length estimate

            let max_len = args
                .get("length")
                .and_then(|v| v.as_i64())
                .map(|v| v as usize)
                .unwrap_or_else(|| {
                    // For text mode, calculate based on terminal width minus prefix
                    width.saturating_sub(prefix_len)
                });

            if content.len() <= max_len {
                Ok(content.into())
            } else {
                // Simple truncation with ellipsis
                if max_len > 1 {
                    Ok(format!("{}…", &content[..max_len.saturating_sub(1)]).into())
                } else {
                    Ok("…".into())
                }
            }
        },
    );
}

fn add_tera_template(tera: &mut Tera, name: &str, body: &str) -> Result<()> {
    if !tera.get_template_names().any(|n| n == name) {
        tera.add_raw_template(name, body)?;
    }
    Ok(())
}

#[derive(Debug, PartialEq, Clone, Copy)]
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
    // Fast path: no tags
    if !s.contains("<clx:flex>") {
        trace!(chars = s.len(), "flex: no flex tags");
        return s.to_string();
    }

    debug!(chars = s.len(), width = width, "flex: processing");
    if s.len() > 100 {
        trace!(first_100_chars = ?&s[..100], "flex: long content preview");
    }

    // Process repeatedly until no tags remain or no progress can be made
    let mut current = s.to_string();
    let max_passes = 8; // avoid pathological loops
    for _ in 0..max_passes {
        if !current.contains("<clx:flex>") {
            break;
        }

        let before = current.clone();
        current = flex_process_once(&before, width);

        if current == before {
            // No progress; bail out
            break;
        }
    }
    current
}

fn flex_process_once(s: &str, width: usize) -> String {
    // Check if we have flex tags that might span multiple lines
    let flex_count = s.matches("<clx:flex>").count();
    trace!(flex_count = flex_count, "flex: tag count");
    if flex_count >= 2 {
        // We have a complete flex tag pair, process as a single unit
        let parts = s.splitn(3, "<clx:flex>").collect::<Vec<_>>();
        trace!(parts_count = parts.len(), "flex: split parts");
        if parts.len() >= 2 {
            let prefix = parts[0];
            let content = parts[1];
            let suffix = if parts.len() == 3 { parts[2] } else { "" };
            trace!(
                prefix = ?prefix,
                content_len = content.len(),
                suffix = ?suffix,
                "flex: parts breakdown"
            );

            // Handle empty content case
            if content.is_empty() {
                let mut result = String::new();
                result.push_str(prefix);
                result.push_str(suffix);
                return result;
            }

            // For multi-line content, we need to handle it specially
            let content_lines: Vec<&str> = content.lines().collect();
            let prefix_lines: Vec<&str> = prefix.lines().collect();
            let suffix_lines: Vec<&str> = suffix.lines().collect();

            // Calculate the width available on the first line
            let first_line_prefix = prefix_lines.last().unwrap_or(&"");
            let first_line_prefix_width = console::measure_text_width(first_line_prefix);

            // For multi-line content, truncate more aggressively
            if content_lines.len() > 1 {
                let available_width = width.saturating_sub(first_line_prefix_width + 3); // ellipsis

                let mut result = String::new();
                result.push_str(prefix);

                if let Some(first_content_line) = content_lines.first() {
                    if available_width > 3 {
                        let truncated =
                            console::truncate_str(first_content_line, available_width, "…");
                        result.push_str(&truncated);
                    } else {
                        result.push('…');
                    }
                } else {
                    result.push_str(content);
                }

                // Intentionally omit suffix for multi-line
                return result;
            } else {
                // Single line with flex tags, process normally
                let suffix_width = if suffix_lines.is_empty() {
                    0
                } else {
                    console::measure_text_width(suffix_lines[0])
                };
                let available_for_content =
                    width.saturating_sub(first_line_prefix_width + suffix_width);

                // If prefix alone exceeds width, truncate everything to fit
                if first_line_prefix_width >= width {
                    return console::truncate_str(prefix, width, "…").to_string();
                }

                let mut result = String::new();
                result.push_str(prefix);

                if available_for_content > 3 {
                    result.push_str(&console::truncate_str(content, available_for_content, "…"));
                    result.push_str(suffix);
                } else {
                    let available = width.saturating_sub(first_line_prefix_width);
                    if available > 3 {
                        result.push_str(&console::truncate_str(content, available, "…"));
                    }
                }

                return result;
            }
        }
    }

    // Fallback: process line by line for incomplete flex tags
    s.lines()
        .map(|line| {
            if !line.contains("<clx:flex>") {
                return line.to_string();
            }

            let parts = line.splitn(3, "<clx:flex>").collect::<Vec<_>>();
            if parts.len() < 2 {
                return line.to_string();
            }

            let prefix = parts[0];
            let content = parts[1];
            let suffix = if parts.len() == 3 { parts[2] } else { "" };

            let prefix_width = console::measure_text_width(prefix);
            let suffix_width = console::measure_text_width(suffix);
            let available_for_content = width.saturating_sub(prefix_width + suffix_width);

            if prefix_width >= width {
                return console::truncate_str(line, width, "…").to_string();
            }

            let mut result = String::new();
            result.push_str(prefix);

            if available_for_content > 3 {
                result.push_str(&console::truncate_str(content, available_for_content, "…"));
                result.push_str(suffix);
            } else {
                let available = width.saturating_sub(prefix_width);
                if available > 3 {
                    result.push_str(&console::truncate_str(content, available, "…"));
                }
            }

            result
        })
        .collect::<Vec<_>>()
        .join("\n")
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

    #[test]
    fn test_flex() {
        // Test normal case
        let s = "prefix<clx:flex>content<clx:flex>suffix";
        let result = flex(s, 20);
        let width = console::measure_text_width(&result);
        println!("Normal case: result='{}', width={}", result, width);
        assert!(width <= 20);
        assert!(result.contains("prefix"));
        assert!(result.contains("suffix"));

        // Test case where prefix + suffix are longer than available width
        let s = "very_long_prefix<clx:flex>content<clx:flex>very_long_suffix";
        let result = flex(s, 10);
        let width = console::measure_text_width(&result);
        println!(
            "Long prefix/suffix case: result='{}', width={}",
            result, width
        );
        assert!(width <= 10);
        // When truncating, we expect the result to be within width limits
        assert!(!result.is_empty());

        // Test case with extremely long content
        let long_content = "a".repeat(1000);
        let s = format!("prefix<clx:flex>{}<clx:flex>suffix", long_content);
        let result = flex(&s, 30);
        let width = console::measure_text_width(&result);
        println!("Long content case: result='{}', width={}", result, width);
        assert!(width <= 30);
        assert!(result.contains("prefix"));
        assert!(result.contains("suffix"));

        // Test case with extremely long prefix and suffix (like the ensembler_stdout issue)
        let long_prefix = "very_long_prefix_that_exceeds_screen_width_".repeat(10);
        let long_suffix = "very_long_suffix_that_exceeds_screen_width_".repeat(10);
        let s = format!("{}<clx:flex>content<clx:flex>{}", long_prefix, long_suffix);
        let result = flex(&s, 50);
        let width = console::measure_text_width(&result);
        println!(
            "Extreme long prefix/suffix case: result='{}', width={}",
            result, width
        );
        assert!(width <= 50);
        // Should still contain some content
        assert!(!result.is_empty());
    }
}
