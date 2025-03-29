use clx::progress;
use ensembler::CmdLineRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // progress::set_output(progress::ProgressOutput::Text);
    let pr = progress::ProgressJobBuilder::new()
        .body(vec!["{{ spinner() }} {{ bin }} {{ stdout }}".to_string()])
        .status(progress::ProgressStatus::Pending)
        .start();
    CmdLineRunner::new("bash")
        .arg("-c")
        .arg("sleep 1; echo 'hello'; sleep 1; echo 'error' >&2; sleep 1; echo 'done'")
        .with_pr(pr.clone())
        .execute()
        .await?;
    Ok(())
}
