use std::path::Path;

use similar::TextDiff;

/// Render a unified diff between two strings
pub fn render_unified_diff(old: &str, new: &str, old_label: &str, new_label: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    diff.unified_diff()
        .context_radius(3)
        .header(old_label, new_label)
        .to_string()
}

/// Render a unified diff using normalized file labels.
pub fn render_file_unified_diff(old: &str, new: &str, path: &Path) -> String {
    let path = if cfg!(windows) {
        path.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().into_owned()
    };
    render_unified_diff(old, new, &format!("a/{path}"), &format!("b/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_file_unified_diff_preserves_posix_separators() {
        let path = Path::new("dir/file.txt");
        let diff = render_file_unified_diff("before\n", "after\n", path);
        assert!(diff.contains("--- a/dir/file.txt"));
        assert!(diff.contains("+++ b/dir/file.txt"));
    }

    #[test]
    #[cfg(windows)]
    fn test_render_file_unified_diff_normalizes_windows_separators() {
        let path = Path::new(r"D:\repo\file.txt");
        let diff = render_file_unified_diff("before\r\n", "after\n", path);
        assert!(diff.contains("--- a/D:/repo/file.txt"));
        assert!(diff.contains("+++ b/D:/repo/file.txt"));
    }
}
