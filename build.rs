mod build {
    pub mod generate_builtins;
    pub mod generate_settings;
}

use std::env;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Generate builtins
    build::generate_builtins::generate(&out_dir)?;

    // Generate settings code if settings.toml exists
    if Path::new("settings.toml").exists() {
        println!("cargo:rerun-if-changed=settings.toml");
        build::generate_settings::generate(&out_dir)?;
    }

    Ok(())
}
