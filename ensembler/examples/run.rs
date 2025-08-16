use clx::progress;
use ensembler::CmdLineRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // progress::set_output(progress::ProgressOutput::Text);
    let pr = progress::ProgressJobBuilder::new()
        .body("{{ spinner() }} {{ bin }} {{ message }}")
        .prop("bin", "echo")
        .status(progress::ProgressStatus::Hide)
        .start();
    CmdLineRunner::new("bash")
        .arg("-c")
        .arg("sleep 1; echo 'hello'; sleep 1; echo 'error' >&2; sleep 1; echo 'done'")
        .with_pr(pr.clone())
        .execute()
        .await?;
    clx::progress::flush();
    Ok(())
}
