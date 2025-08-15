use crate::Result;
use crate::step::Step;
use serde::Serialize;
use std::sync::Arc;
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex as StdMutex, time::Instant};

#[derive(Debug)]
pub struct TimingRecorder {
    start_instant: Instant,
    intervals_by_step: StdMutex<BTreeMap<String, Vec<(u128, u128)>>>,
    step_profiles: BTreeMap<String, Vec<String>>,
    step_interactive: BTreeMap<String, bool>,
    output_path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Clone)]
struct TimingReportTotal {
    wall_time_ms: u128,
}

#[derive(Debug, Serialize, Clone)]
struct TimingReportJson {
    total: TimingReportTotal,
    steps: BTreeMap<String, TimingReportStep>,
}

#[derive(Debug, Serialize, Clone)]
struct TimingReportStep {
    wall_time_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    profiles: Option<Vec<String>>,
    interactive: bool,
}

impl TimingRecorder {
    pub fn new(output_path: Option<PathBuf>) -> Self {
        Self {
            start_instant: Instant::now(),
            intervals_by_step: StdMutex::new(BTreeMap::new()),
            step_profiles: BTreeMap::new(),
            step_interactive: BTreeMap::new(),
            output_path,
        }
    }

    pub fn now_ms(&self) -> u128 {
        self.start_instant.elapsed().as_millis()
    }

    pub fn add_interval(&self, step: &str, start_ms: u128, end_ms: u128) {
        if end_ms < start_ms {
            return;
        }
        let mut map = self.intervals_by_step.lock().unwrap();
        map.entry(step.to_string())
            .or_default()
            .push((start_ms, end_ms));
    }

    pub fn set_step_profiles(&mut self, step_name: &str, profiles: Option<&[String]>) {
        if let Some(p) = profiles {
            self.step_profiles.insert(step_name.to_string(), p.to_vec());
        } else {
            self.step_profiles.remove(step_name);
        }
    }

    pub fn set_step_interactive(&mut self, step_name: &str, interactive: bool) {
        self.step_interactive
            .insert(step_name.to_string(), interactive);
    }

    fn merge_and_sum(intervals: &mut [(u128, u128)]) -> u128 {
        if intervals.is_empty() {
            return 0;
        }
        intervals.sort_by_key(|(s, e)| (*s, *e));
        let mut total: u128 = 0;
        let mut cur = intervals[0];
        for &(s, e) in intervals.iter().skip(1) {
            if s <= cur.1 {
                if e > cur.1 {
                    cur.1 = e;
                }
            } else {
                total += cur.1 - cur.0;
                cur = (s, e);
            }
        }
        total += cur.1 - cur.0;
        total
    }

    fn build_report(&self) -> TimingReportJson {
        let elapsed_ms = self.start_instant.elapsed().as_millis();
        let mut steps: BTreeMap<String, TimingReportStep> = BTreeMap::new();
        let mut map = self.intervals_by_step.lock().unwrap();
        for (name, intervals) in map.iter_mut() {
            let wall_ms = Self::merge_and_sum(intervals.as_mut_slice());
            let profiles = self.step_profiles.get(name).cloned();
            let interactive = self.step_interactive.get(name).cloned().unwrap_or(false);
            steps.insert(
                name.clone(),
                TimingReportStep {
                    wall_time_ms: wall_ms,
                    profiles,
                    interactive,
                },
            );
        }
        TimingReportJson {
            total: TimingReportTotal {
                wall_time_ms: elapsed_ms,
            },
            steps,
        }
    }

    pub fn write_json(&self) -> Result<()> {
        let Some(output_path) = &self.output_path else {
            return Ok(());
        };
        let json = self.build_report();
        let data = serde_json::to_vec_pretty(&json)?;
        if let Some(parent) = output_path.parent() {
            xx::file::mkdirp(parent)?;
        }
        xx::file::write(output_path, &data)?;
        Ok(())
    }

    pub fn to_json_string(&self) -> Result<String> {
        let json = self.build_report();
        let s = serde_json::to_string_pretty(&json)?;
        Ok(s)
    }
}

#[derive(Debug)]
pub struct StepTimingGuard {
    recorder: Arc<TimingRecorder>,
    step_name: String,
    start_ms: u128,
}

impl StepTimingGuard {
    pub fn new(recorder: Arc<TimingRecorder>, step: &Step) -> Self {
        let start_ms = recorder.now_ms();
        Self {
            recorder,
            step_name: step.name.clone(),
            start_ms,
        }
    }

    pub fn finish(self) {
        let end_ms = self.recorder.now_ms();
        self.recorder
            .add_interval(&self.step_name, self.start_ms, end_ms);
    }
}
