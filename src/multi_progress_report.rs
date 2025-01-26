use crate::Result;
use std::sync::{Arc, Mutex, Weak};

use indicatif::MultiProgress;

use crate::progress_report::{ProgressReport, QuietReport, SingleReport, VerboseReport};

#[derive(Debug)]
pub enum OutputType {
    ProgressBar,
    Verbose,
    Quiet,
}

#[derive(Debug)]
pub struct MultiProgressReport {
    mp: Option<MultiProgress>,
    output_type: OutputType,
}

static INSTANCE: Mutex<Option<Weak<MultiProgressReport>>> = Mutex::new(None);

impl MultiProgressReport {
    fn try_get() -> Option<Arc<Self>> {
        match &*INSTANCE.lock().unwrap() {
            Some(w) => w.upgrade(),
            None => None,
        }
    }
    pub fn set_output_type(output_type: OutputType) {
        let mut mutex = INSTANCE.lock().unwrap();
        let mpr = Arc::new(Self::new(output_type));
        *mutex = Some(Arc::downgrade(&mpr));
    }
    pub fn get() -> Arc<Self> {
        let mut mutex = INSTANCE.lock().unwrap();
        if let Some(w) = &*mutex {
            if let Some(mpr) = w.upgrade() {
                return mpr;
            }
        }
        let output_type = if !console::user_attended_stderr() {
            OutputType::Quiet
        } else {
            OutputType::ProgressBar
        };
        let mpr = Arc::new(Self::new(output_type));
        *mutex = Some(Arc::downgrade(&mpr));
        mpr
    }
    fn new(output_type: OutputType) -> Self {
        let mp = match output_type {
            OutputType::ProgressBar => Some(MultiProgress::new()),
            _ => None,
        };
        MultiProgressReport { mp, output_type }
    }
    pub fn add(&self, prefix: &str) -> Box<dyn SingleReport> {
        match self.output_type {
            OutputType::ProgressBar => {
                let mut pr = ProgressReport::new(prefix.into());
                if let Some(mp) = &self.mp {
                    pr.pb = mp.add(pr.pb);
                }
                Box::new(pr)
            }
            OutputType::Quiet => Box::new(QuietReport::new()),
            OutputType::Verbose => Box::new(VerboseReport::new(prefix.to_string())),
        }
    }
    pub fn suspend_if_active<F: FnOnce() -> R, R>(f: F) -> R {
        match Self::try_get() {
            Some(mpr) => mpr.suspend(f),
            None => f(),
        }
    }
    pub fn suspend<F: FnOnce() -> R, R>(&self, f: F) -> R {
        match &self.mp {
            Some(mp) => mp.suspend(f),
            None => f(),
        }
    }
    pub fn stop(&self) -> Result<()> {
        if let Some(mp) = &self.mp {
            mp.clear()?;
        }
        Ok(())
    }
}
