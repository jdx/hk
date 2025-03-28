use std::{thread, time::Duration};

use clx::progress::{ProgressBuilder, ProgressStatus};

#[tokio::main]
async fn main() {
    let root = ProgressBuilder::new("root".to_string()).build();
    ProgressBuilder::new("root-pending".to_string())
        .status(ProgressStatus::Pending)
        .build();
    let root2 = ProgressBuilder::new("root-2".to_string()).build();
    for i in 0..3 {
        thread::sleep(Duration::from_millis(100));
        let pb = ProgressBuilder::new(format!("test {}", i));
        root.add(pb);
    }
    thread::sleep(Duration::from_secs(1));
    root.set_status(ProgressStatus::Done);
    thread::sleep(Duration::from_millis(300));
    root2.set_status(ProgressStatus::Done);
    thread::sleep(Duration::from_millis(100));
}
