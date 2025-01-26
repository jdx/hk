use ensembler::CmdLineRunner;

fn main() -> ensembler::Result<()> {
    let mpr = ensembler::MultiProgressReport::get();
    CmdLineRunner::new("sleep")
        .arg("1")
        .with_pr(mpr.add("sleeping").into())
        .execute()?;

    Ok(())
}
