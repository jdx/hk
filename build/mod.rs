pub mod embed_pkl_package;
pub mod generate_builtins;
pub mod generate_settings;
pub mod settings_toml;

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Rerun if source data changes
    println!("cargo:rerun-if-changed=build/");
    println!("cargo:rerun-if-changed=pkl");
    println!("cargo:rerun-if-changed=pkl/builtins");
    println!("cargo:rerun-if-changed=pkl/builtins_meta.json");
    println!("cargo:rerun-if-changed=settings.toml");
    println!("cargo:rerun-if-env-changed=HK_REQUIRE_EMBEDDED_PKL");

    generate_builtins::generate(&out_dir)?;
    generate_settings::generate(&out_dir)?;
    embed_pkl_package::generate(&out_dir)?;
    link_without_pie();

    Ok(())
}

/// Release builds for Linux GNU set `HK_NO_PIE=1` (see
/// .github/workflows/release.yml) to link the executable at a fixed address. As
/// a position-independent executable, hk makes the dynamic loader patch about
/// 70k pointers on every launch, which copies hundreds of pages and is a large
/// share of the run time of a git hook with nothing to do. Linked non-PIE, those
/// pointers are final in the file. Dependencies are still compiled
/// position-independent; the flag reaches only bin targets, so no shared
/// library is linked with it.
fn link_without_pie() {
    println!("cargo:rerun-if-env-changed=HK_NO_PIE");
    let linux_gnu = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu");
    if linux_gnu && env::var("HK_NO_PIE").as_deref() == Ok("1") {
        println!("cargo:rustc-link-arg-bins=-no-pie");
    }
}
