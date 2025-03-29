use crate::style;

pub(crate) fn progress_bar(progress_current: usize, progress_total: usize, width: usize) -> String {
    let width = width - 2;
    let progress = progress_current as f64 / progress_total as f64;
    let filled_length = (width as f64 * progress).round() as usize;
    let progress_bar = "=".repeat(filled_length) + &" ".repeat(width - filled_length);
    style::edim(format!("[{}]", progress_bar)).to_string()
}
