pub mod generate_builtins;
pub mod generate_settings;
pub mod settings_toml;

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Rerun if source data changes
    println!("cargo:rerun-if-changed=build/");
    println!("cargo:rerun-if-changed=pkl/builtins");
    println!("cargo:rerun-if-changed=settings.toml");

    generate_builtins::generate(&out_dir)?;
    generate_settings::generate(&out_dir)?;

    Ok(())
}
