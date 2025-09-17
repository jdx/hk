// Include generated code from build.rs
include!(concat!(env!("OUT_DIR"), "/generated_settings.rs"));

pub mod cli {
    include!(concat!(env!("OUT_DIR"), "/generated_cli_flags.rs"));
}

pub mod git {
    include!(concat!(env!("OUT_DIR"), "/generated_git_keys.rs"));
}
