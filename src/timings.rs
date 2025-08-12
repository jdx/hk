use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex as StdMutex, time::Instant};

#[derive(Debug)]
pub struct TimingRecorder {
    start_instant: Instant,
    intervals_by_step: StdMutex<BTreeMap<String, Vec<(u128, u128)>>>,
    step_profiles: StdMutex<BTreeMap<String, Vec<String>>>,
    output_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct TimingReportTotal {
    wall_time_ms: u128,
}

#[derive(Debug, Serialize)]
struct TimingReportJson {
    total: TimingReportTotal,
    steps: BTreeMap<String, TimingReportStep>,
}

#[derive(Debug, Serialize)]
struct TimingReportStep {
    wall_time_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    profiles: Option<Vec<String>>,
}

impl TimingRecorder {
    pub fn new(output_path: PathBuf) -> Self {
        Self {
            start_instant: Instant::now(),
            intervals_by_step: StdMutex::new(BTreeMap::new()),
            step_profiles: StdMutex::new(BTreeMap::new()),
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

    pub fn write_json(&self) -> crate::Result<()> {
        if let Some(parent) = self.output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let elapsed_ms = self.start_instant.elapsed().as_millis();
        let mut steps: BTreeMap<String, TimingReportStep> = BTreeMap::new();
        let mut map = self.intervals_by_step.lock().unwrap();
        let profiles_map = self.step_profiles.lock().unwrap();
        for (name, intervals) in map.iter_mut() {
            let wall_ms = Self::merge_and_sum(intervals.as_mut_slice());
            let profiles = profiles_map.get(name).cloned();
            steps.insert(
                name.clone(),
                TimingReportStep {
                    wall_time_ms: wall_ms,
                    profiles,
                },
            );
        }
        let json = TimingReportJson {
            total: TimingReportTotal {
                wall_time_ms: elapsed_ms,
            },
            steps,
        };
        let data = serde_json::to_vec_pretty(&json)?;
        std::fs::write(&self.output_path, data)?;
        Ok(())
    }
}
