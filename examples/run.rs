use clx::{MultiProgressReport, OutputType};
use ensembler::CmdLineRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    MultiProgressReport::set_output_type(OutputType::Verbose);
    let mpr = MultiProgressReport::get();
    let pr = mpr.add("ls");
    CmdLineRunner::new("ls")
        .with_pr(pr.clone())
        .execute()
        .await?;
    Ok(())
}
