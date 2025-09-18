use codegen::Scope;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn generate(out_dir: &Path) -> Result<(), std::io::Error> {
    let builtins_dir = Path::new("pkl/builtins");

    // Collect all .pkl files in the builtins directory
    let builtins = collect_builtin_files(builtins_dir)?;

    // Generate the builtins module using Scope
    let mut scope = Scope::new();

    // Create the BUILTINS constant as a static array
    let builtins_array = generate_builtins_array(&builtins);
    scope.raw(&builtins_array);

    // Write to file
    let dest_path = out_dir.join("builtins.rs");
    fs::write(dest_path, scope.to_string())?;

    Ok(())
}

fn collect_builtin_files(builtins_dir: &Path) -> Result<BTreeSet<String>, std::io::Error> {
    let mut builtins = BTreeSet::new();

    for entry in fs::read_dir(builtins_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("pkl") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                builtins.insert(name.to_string());
            }
        }
    }

    Ok(builtins)
}

fn generate_builtins_array(builtins: &BTreeSet<String>) -> String {
    let items: Vec<String> = builtins.iter().map(|b| format!("    \"{}\"", b)).collect();

    format!(
        "/// List of all available builtin configurations\npub const BUILTINS: &[&str] = &[\n{},\n];",
        items.join(",\n")
    )
}
