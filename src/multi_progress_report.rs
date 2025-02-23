use crate::Result;
use std::sync::{Arc, Mutex};

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

static INSTANCE: Mutex<Option<Arc<MultiProgressReport>>> = Mutex::new(None);
pub(crate) static CLI_NAME: Mutex<Option<String>> = Mutex::new(None);

impl MultiProgressReport {
    fn try_get() -> Option<Arc<Self>> {
        INSTANCE.lock().unwrap().clone()
    }
    pub fn set_output_type(output_type: OutputType) {
        let mut mutex = INSTANCE.lock().unwrap();
        let mpr = Arc::new(Self::new(output_type));
        *mutex = Some(mpr);
    }
    pub fn set_cli_name(cli_name: &str) {
        let mut mutex = CLI_NAME.lock().unwrap();
        *mutex = Some(cli_name.to_string());
    }
    pub fn get() -> Arc<Self> {
        let mut mutex = INSTANCE.lock().unwrap();
        if let Some(mpr) = &*mutex {
            return mpr.clone();
        }
        let output_type = if !console::user_attended_stderr() {
            OutputType::Quiet
        } else {
            OutputType::ProgressBar
        };
        let mpr = Arc::new(Self::new(output_type));
        *mutex = Some(mpr.clone());
        mpr
    }
    fn new(output_type: OutputType) -> Self {
        let mp = match output_type {
            OutputType::ProgressBar => Some(MultiProgress::new()),
            _ => None,
        };
        MultiProgressReport { mp, output_type }
    }
    pub fn add(&self, prefix: &str) -> Arc<Box<dyn SingleReport>> {
        match self.output_type {
            OutputType::ProgressBar => {
                let mut pr = ProgressReport::new(prefix.into());
                if let Some(mp) = &self.mp {
                    pr.pb = mp.add(pr.pb);
                }
                Arc::new(Box::new(pr))
            }
            OutputType::Quiet => Arc::new(Box::new(QuietReport::new())),
            OutputType::Verbose => Arc::new(Box::new(VerboseReport::new(prefix.to_string()))),
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
