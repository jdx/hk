use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn generate(out_dir: &Path) -> Result<(), std::io::Error> {
    let dest_path = out_dir.join("builtins.rs");

    let builtins_dir = Path::new("pkl/builtins");
    let builtins = ls(builtins_dir)?
        .into_iter()
        .filter_map(|f| f.strip_suffix(".pkl").map(|s| s.to_string()))
        .collect::<BTreeSet<String>>();

    let code = format!(
        "pub const BUILTINS: &[&str] = &[{}];",
        builtins
            .iter()
            .map(|b| format!("\"{b}\""))
            .collect::<Vec<String>>()
            .join(", ")
    );

    fs::write(dest_path, code)?;
    println!("cargo:rerun-if-changed=pkl/builtins");
    Ok(())
}

fn ls(path: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        files.push(file_name.to_string_lossy().to_string());
    }
    Ok(files)
}
