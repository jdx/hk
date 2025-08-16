use crate::style;

pub(crate) fn progress_bar(progress_current: usize, progress_total: usize, width: usize) -> String {
    let width = width.max(2) - 2;
    let progress = progress_current as f64 / progress_total as f64;
    let filled_length = (width as f64 * progress).round() as usize;
    let progress_bar = if progress == 1.0 {
        "=".repeat(width)
    } else if filled_length > 0 {
        "=".repeat(filled_length - 1) + ">" + &" ".repeat(width - filled_length)
    } else {
        " ".repeat(width)
    };
    style::edim(format!("[{}]", progress_bar)).to_string()
}
