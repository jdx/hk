use std::{thread, time::Duration};

use clx::progress::{ProgressJobBuilder, ProgressStatus};

#[tokio::main]
async fn main() {
    // clx::progress::set_output(clx::progress::ProgressOutput::Text);
    let root = ProgressJobBuilder::new().prop("message", "root")
    .on_done(clx::progress::ProgressJobDoneBehavior::Collapse)
    .start();
    ProgressJobBuilder::new()
        .prop("message", "pending")
        .status(ProgressStatus::Pending)
        .start();
    let root2 = ProgressJobBuilder::new().prop("message", "root2").start();
    let root3 = ProgressJobBuilder::new().prop("message", "root3").start();
    for i in 0..3 {
        thread::sleep(Duration::from_millis(100));
        let pb = ProgressJobBuilder::new().prop("message", &format!("running {}", i)).build();
        root.add(pb);
    }
    thread::sleep(Duration::from_secs(1));
    root.set_status(ProgressStatus::Done);
    root3.set_status(ProgressStatus::Failed);
    thread::sleep(Duration::from_millis(300));
    root2.set_status(ProgressStatus::Done);
    thread::sleep(Duration::from_millis(100));
}
