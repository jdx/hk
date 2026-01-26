use crate::Result;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::{io::Write, sync::OnceLock};

use crate::{env, ui};
use clx::progress;
use log::{Level, LevelFilter, Metadata, Record};

#[derive(Debug)]
struct Logger {
    level: LevelFilter,
    term_level: LevelFilter,
    file_level: LevelFilter,
    log_file: Option<Mutex<File>>,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if record.level() <= self.file_level
            && let Some(log_file) = &self.log_file
        {
            let mut log_file = log_file.lock().unwrap();
            let out = format!(
                "{now} {level} {args}",
                now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                level = self.styled_level(record.level()),
                args = record.args()
            );
            let _ = writeln!(log_file, "{}", console::strip_ansi_codes(&out));
        }
        if record.level() <= self.term_level {
            let out = self.render(record, self.term_level);
            if !out.is_empty() {
                progress::pause();
                eprintln!("{out}");
                progress::resume();
            }
        }
    }

    fn flush(&self) {}
}

impl Logger {
    fn init(level: Option<LevelFilter>) -> Self {
        let term_level = level.unwrap_or(*env::HK_LOG);
        let file_level = *env::HK_LOG_FILE_LEVEL;

        let mut logger = Logger {
            level: std::cmp::max(term_level, file_level),
            file_level,
            term_level,
            log_file: None,
        };

        let log_file = &*env::HK_LOG_FILE;
        if let Ok(log_file) = init_log_file(log_file) {
            logger.log_file = Some(Mutex::new(log_file));
        } else {
            warn!("could not open log file: {log_file:?}");
        }

        logger
    }

    fn render(&self, record: &Record, level: LevelFilter) -> String {
        let ignore_crates = ["/notify-debouncer-full-", "/notify-", "/globset-"];
        if record.level() <= LevelFilter::Debug
            && ignore_crates
                .iter()
                .any(|c| record.file().unwrap_or("<unknown>").contains(c))
        {
            return "".to_string();
        }
        match level {
            LevelFilter::Off => "".to_string(),
            LevelFilter::Trace => {
                let file = record.file().unwrap_or("<unknown>");
                let meta = ui::style::edim(format!(
                    "{thread_id:>2} [{file}:{line}]",
                    thread_id = thread_id(),
                    line = record.line().unwrap_or(0),
                ));
                format!(
                    "{level} {meta} {args}",
                    level = self.styled_level(record.level()),
                    args = record.args()
                )
            }
            LevelFilter::Debug => format!(
                "{level} {args}",
                level = self.styled_level(record.level()),
                args = record.args()
            ),
            _ => {
                let hk = match record.level() {
                    Level::Error => ui::style::ered("hk"),
                    Level::Warn => ui::style::eyellow("hk"),
                    _ => ui::style::edim("hk"),
                };
                match record.level() {
                    Level::Info => format!("{hk} {args}", args = record.args()),
                    _ => format!(
                        "{hk} {level} {args}",
                        level = self.styled_level(record.level()),
                        args = record.args()
                    ),
                }
            }
        }
    }

    fn styled_level(&self, level: Level) -> String {
        let level = match level {
            Level::Error => ui::style::ered("ERROR").to_string(),
            Level::Warn => ui::style::eyellow("WARN").to_string(),
            Level::Info => ui::style::ecyan("INFO").to_string(),
            Level::Debug => ui::style::emagenta("DEBUG").to_string(),
            Level::Trace => ui::style::edim("TRACE").to_string(),
        };
        console::pad_str(&level, 5, console::Alignment::Left, None).to_string()
    }
}

pub fn thread_id() -> String {
    let id = format!("{:?}", thread::current().id());
    let id = id.replace("ThreadId(", "");
    id.replace(")", "")
}

pub fn init(level: Option<LevelFilter>) {
    static LOGGER: OnceLock<Logger> = OnceLock::new();
    let logger = LOGGER.get_or_init(|| Logger::init(level));
    if let Err(err) = log::set_logger(logger).map(|()| log::set_max_level(logger.level)) {
        eprintln!("mise: could not initialize logger: {err}");
    }
}

fn init_log_file(log_file: &Path) -> Result<File> {
    if let Some(log_dir) = log_file.parent() {
        xx::file::mkdirp(log_dir)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    Ok(file)
}
