use crate::Result;
use crate::step::Step;
use serde::Serialize;
use std::sync::Arc;
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex as StdMutex, time::Instant};

#[derive(Debug)]
pub struct TimingRecorder {
    start_instant: Instant,
    intervals_by_step: StdMutex<BTreeMap<String, Vec<(u128, u128)>>>,
    step_profiles: StdMutex<BTreeMap<String, Vec<String>>>,
    step_interactive: StdMutex<BTreeMap<String, bool>>,
    output_path: PathBuf,
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
    pub fn new(output_path: PathBuf) -> Self {
        Self {
            start_instant: Instant::now(),
            intervals_by_step: StdMutex::new(BTreeMap::new()),
            step_profiles: StdMutex::new(BTreeMap::new()),
            step_interactive: StdMutex::new(BTreeMap::new()),
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

    pub fn record_profiles(&self, step_name: &str, profiles: Option<&[String]>) {
        if let Some(p) = profiles {
            let mut map = self.step_profiles.lock().unwrap();
            map.entry(step_name.to_string())
                .or_insert_with(|| p.to_vec());
        }
    }

    pub fn record_interactive(&self, step_name: &str, interactive: bool) {
        let mut map = self.step_interactive.lock().unwrap();
        map.insert(step_name.to_string(), interactive);
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
        if let Some(parent) = self.output_path.parent() {
            let _ = xx::file::mkdirp(parent);
        }
        let elapsed_ms = self.start_instant.elapsed().as_millis();
        let mut steps: BTreeMap<String, TimingReportStep> = BTreeMap::new();
        let mut map = self.intervals_by_step.lock().unwrap();
        let profiles_map = self.step_profiles.lock().unwrap();
        let interactive_map = self.step_interactive.lock().unwrap();
        for (name, intervals) in map.iter_mut() {
            let wall_ms = Self::merge_and_sum(intervals.as_mut_slice());
            let profiles = profiles_map.get(name).cloned();
            let interactive = interactive_map.get(name).cloned().unwrap_or(false);
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
        let json = self.build_report();
        let data = serde_json::to_vec_pretty(&json)?;
        xx::file::write(&self.output_path, &data)?;
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
        if let Some(p) = step.profiles.as_ref() {
            recorder.record_profiles(&step.name, Some(p));
        }
        recorder.record_interactive(&step.name, step.interactive);
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
