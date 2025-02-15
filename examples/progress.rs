use clx::MultiProgressReport;

#[tokio::main]
async fn main() {
    let mpr = MultiProgressReport::get();
    let pr = mpr.add("test");
    for i in 0..100 {
        pr.set_message(format!("{}", i));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
