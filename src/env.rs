pub use std::env::*;
use itertools::Itertools;
use std::sync::LazyLock;

#[cfg(test)]
pub static TERM_WIDTH: LazyLock<usize> = LazyLock::new(|| 80);

#[cfg(not(test))]
pub static TERM_WIDTH: LazyLock<usize> = LazyLock::new(|| {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(80)
        .max(80)
});

pub static PATH_KEY: LazyLock<String> = LazyLock::new(|| {
    vars()
        .map(|(k, _)| k)
        .find_or_first(|k| k.to_uppercase() == "PATH")
        .map(|k| k.to_string())
        .unwrap_or("PATH".into())
});
