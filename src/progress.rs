use crate::Result;
use serde::ser::Serialize;
use std::{
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use console::Term;
use indicatif::TermLike;
use tera::{Context, Tera};
pub struct Job {
    // id: String,
    name: String,
    body: Mutex<String>,
    done: AtomicBool,
    children: Mutex<Vec<Arc<Job>>>,
    tera_ctx: Mutex<Context>,
}

const DEFAULT_BODY: &str = "{{ spinner }} {{ name }}\n{{ body }}";
const SPINNER: &str = "⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈";

#[derive(Clone, Default)]
struct RenderContext {
    tera_ctx: Context,
    indent: usize,
}

impl Job {
    pub fn new(name: String) -> Self {
        // static INC: AtomicUsize = AtomicUsize::new(0);
        // let inc = INC.fetch_add(1, Ordering::Relaxed);
        Job {
            // id: format!("{name}-{inc}"),
            name,
            body: Mutex::new(DEFAULT_BODY.to_string()),
            done: Default::default(),
            children: Default::default(),
            tera_ctx: Default::default(),
        }
    }

    pub fn root() -> &'static Self {
        static ROOT: LazyLock<Job> = LazyLock::new(|| Job::new("root".to_string()));
        &ROOT
    }

    pub fn interval() -> Duration {
        INTERVAL.lock().unwrap().clone()
    }

    pub fn set_interval(interval: Duration) {
        *INTERVAL.lock().unwrap() = interval;
    }

    pub fn display() {
        thread::spawn(move || {
            let mut tera = Tera::default();
            let mut ctx = RenderContext::default();
            ctx.tera_ctx.insert("body", "");
            loop {
                let root = Self::root();
                if let Err(err) = Self::refresh(&root, &mut tera, ctx.clone()) {
                    eprintln!("clx: {:?}", err);
                    *LINES.lock().unwrap() = 0;
                }
                if root.is_done() {
                    // TODO: show completion version of output
                    return;
                }
                thread::sleep(Self::interval());
            }
        });
    }

    fn refresh(root: &Job, tera: &mut Tera, mut ctx: RenderContext) -> Result<()> {
        let term = Self::term();
        let lines = *LINES.lock().unwrap();
        ctx.tera_ctx.insert("spinner", &root.spinner());
        let output = root.render(tera, &ctx)?;
        term.move_cursor_up(lines)?;
        term.clear_to_end_of_screen()?;
        *LINES.lock().unwrap() = output.len();
        for line in output {
            let width = term.width() as usize;
            let line = console::truncate_str(&line, width, "…");
            term.write_line(&line)?;
        }
        Ok(())
    }

    fn render(&self, tera: &mut Tera, ctx: &RenderContext) -> Result<Vec<String>> {
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
            let child_output = child
                .render(tera, &ctx)?
                .into_iter()
                .map(|line| format!("{}{line}", " ".repeat(ctx.indent)));
            s.extend(child_output);
        }
        Ok(s)
    }

    fn term() -> &'static Term {
        static TERM: LazyLock<Term> = LazyLock::new(|| Term::stderr());
        &TERM
    }

    pub fn add(&self, name: String) -> Arc<Self> {
        let job = Arc::new(Job::new(name));
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
    }

    pub fn add_prop<T: Serialize + ?Sized, S: Into<String>>(&mut self, key: S, val: &T) {
        let mut ctx = self.tera_ctx.lock().unwrap();
        ctx.insert(key, val);
    }

    fn spinner(&self) -> char {
        static INC: AtomicUsize = AtomicUsize::new(0);
        let inc = INC.fetch_add(1, Ordering::Relaxed);
        SPINNER.chars().nth(inc % SPINNER.len()).unwrap()
    }
}

static INTERVAL: Mutex<Duration> = Mutex::new(Duration::from_millis(100));
static LINES: Mutex<usize> = Mutex::new(0);
