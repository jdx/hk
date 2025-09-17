mod build {
    pub mod generate_builtins;
    pub mod generate_settings;
}

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=pkl/builtins");
    build::generate_builtins::generate(&out_dir)?;

    println!("cargo:rerun-if-changed=settings.toml");
    build::generate_settings::generate(&out_dir)?;

    Ok(())
}
