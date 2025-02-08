use ensembler::CmdLineRunner;

#[tokio::main]
async fn main() -> ensembler::Result<()> {
    let mpr = ensembler::MultiProgressReport::get();
    let result = CmdLineRunner::new("sleep")
        .arg("1")
        .with_pr(mpr.add("sleeping").into())
        .execute().await?;

    Ok(())
}
