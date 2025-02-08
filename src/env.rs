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
