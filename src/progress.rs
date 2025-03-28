use crate::Result;
use serde::ser::Serialize;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering}, mpsc, Arc, LazyLock, Mutex
    },
    thread,
    time::Duration,
};

use console::Term;
use indicatif::TermLike;
use tera::{Context, Tera};

pub struct ProgressJob {
    // id: String,
    name: String,
    body: Mutex<String>,
    done: AtomicBool,
    children: Mutex<Vec<Arc<ProgressJob>>>,
    tera_ctx: Mutex<Context>,
}

const DEFAULT_BODY: &str = "{{ spinner }} {{ name }}\n{{ body }}";
const SPINNER: &str = "⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈";

static INTERVAL: Mutex<Duration> = Mutex::new(Duration::from_millis(100));
static LINES: Mutex<usize> = Mutex::new(0);
static NOTIFY: Mutex<Option<mpsc::Sender<()>>> = Mutex::new(None);
static PAUSED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Default)]
struct RenderContext {
    width: usize,
    tera_ctx: Context,
    indent: usize,
}

impl ProgressJob {
    pub fn new(name: String) -> Self {
        Self::start();
        ProgressJob {
            // id: format!("{name}-{inc}"),
            name,
            body: Mutex::new(DEFAULT_BODY.to_string()),
            done: Default::default(),
            children: Default::default(),
            tera_ctx: Default::default(),
        }
    }

    pub fn root() -> &'static Self {
        static ROOT: LazyLock<ProgressJob> = LazyLock::new(|| ProgressJob::new("root".to_string()));
        &ROOT
    }

    pub fn interval() -> Duration {
        INTERVAL.lock().unwrap().clone()
    }

    pub fn set_interval(interval: Duration) {
        *INTERVAL.lock().unwrap() = interval;
    }

    pub fn start() {
        static STARTED: Mutex<bool> = Mutex::new(false);
        let mut started = STARTED.lock().unwrap();
        if *started {
            return;
        }
        thread::spawn(move || {
            let mut tera = Tera::default();
            let mut ctx = RenderContext::default();
            ctx.tera_ctx.insert("body", "");
            loop {
                ctx.width = Self::term().width() as usize;
                let root = Self::root();
                if let Err(err) = Self::refresh(&root, &mut tera, ctx.clone()) {
                    eprintln!("clx: {:?}", err);
                    *LINES.lock().unwrap() = 0;
                }
                if root.is_done() {
                    *STARTED.lock().unwrap() = false;
                    return;
                }
                notify_wait(Self::interval());
            }
        });
        *started = true;
    }

    pub fn clear() -> Result<()> {
        let term = Self::term();
        let mut lines = LINES.lock().unwrap();
        term.move_cursor_up(*lines)?;
        term.clear_to_end_of_screen()?;
        *lines = 0;
        Ok(())
    }

    pub fn is_paused() -> bool {
        PAUSED.load(Ordering::Relaxed)
    }

    pub fn pause() {
        PAUSED.store(true, Ordering::Relaxed);
        let _ = Self::clear();
    }

    pub fn resume() {
        PAUSED.store(false, Ordering::Relaxed);
        notify();
    }

    fn refresh(root: &ProgressJob, tera: &mut Tera, mut ctx: RenderContext) -> Result<()> {
        if Self::is_paused() {
            return Ok(());
        }
        let term = Self::term();
        let mut lines = LINES.lock().unwrap();
        ctx.tera_ctx.insert("spinner", &root.spinner());
        let output = root.render(tera, &ctx)?;
        term.move_cursor_up(*lines)?;
        term.clear_to_end_of_screen()?;
        term.write_line(&output)?;
        *lines = output.split("\n").fold(0, |acc, line| {
            acc + 1 + console::measure_text_width(line) / ctx.width
        });
        Ok(())
    }

    fn render(&self, tera: &mut Tera, ctx: &RenderContext) -> Result<String> {
        let mut s = vec![];
        let mut ctx = ctx.clone();
        ctx.tera_ctx.extend(self.tera_ctx.lock().unwrap().clone());
        ctx.tera_ctx.insert("name", &self.name);
        if self.is_done() {
            ctx.tera_ctx.insert("spinner", &"✔");
        }
        let body = tera.render_str(&self.body(), &ctx.tera_ctx)?;
        s.push(body.trim_end().to_string());
        ctx.indent += 2;
        let children = self.children.lock().unwrap();
        for child in children.iter() {
            let child_output = child.render(tera, &ctx)?;
            let child_output = indent(child_output, ctx.width, ctx.indent);
            s.push(child_output);
        }
        Ok(s.join("\n"))
    }

    fn term() -> &'static Term {
        static TERM: LazyLock<Term> = LazyLock::new(|| Term::stderr());
        &TERM
    }

    pub fn add(&self, name: String) -> Arc<Self> {
        let job = Arc::new(ProgressJob::new(name));
        self.children.lock().unwrap().push(job.clone());
        job
    }

    pub fn set_body(&self, body: String) {
        *self.body.lock().unwrap() = body;
    }

    pub fn body(&self) -> String {
        self.body.lock().unwrap().clone()
    }

    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Relaxed)
    }

    pub fn done(&self) {
        self.done.store(true, Ordering::Relaxed);
        notify();
    }

    pub fn add_prop<T: Serialize + ?Sized, S: Into<String>>(&self, key: S, val: &T) {
        let mut ctx = self.tera_ctx.lock().unwrap();
        ctx.insert(key, val);
    }

    fn spinner(&self) -> char {
        static INC: AtomicUsize = AtomicUsize::new(0);
        let inc = INC.fetch_add(1, Ordering::Relaxed);
        SPINNER.chars().nth(inc % SPINNER.len()).unwrap()
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

fn notify_wait(timeout: Duration) {
    let (tx, rx) = mpsc::channel();
    NOTIFY.lock().unwrap().replace(tx);
    let _ = rx.recv_timeout(timeout);
}