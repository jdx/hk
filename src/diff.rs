use similar::TextDiff;

/// Render a unified diff between two strings
pub fn render_unified_diff(old: &str, new: &str, old_label: &str, new_label: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    diff.unified_diff()
        .context_radius(3)
        .header(old_label, new_label)
        .to_string()
}
