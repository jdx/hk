use std::{thread, time::Duration};

use clx::Job;

#[tokio::main]
async fn main() {
    let root = Job::root();
    Job::display();
    for i in 0..3 {
        thread::sleep(Duration::from_millis(100));
        root.add(format!("test {}", i));
    }
    thread::sleep(Duration::from_secs(1));
    root.done();
    thread::sleep(Duration::from_millis(100));
}
