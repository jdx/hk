#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use std::path::PathBuf;
    use indexmap::IndexSet;

    #[test]
    fn test_exclude_paths_union() {
        // Test that exclude paths are properly unioned from multiple sources
        Settings::add_exclude_paths(vec![PathBuf::from("node_modules")]);
        Settings::add_exclude_paths(vec![PathBuf::from("target")]);

        let settings = Settings::get();
        assert!(settings.exclude_paths.contains(&PathBuf::from("node_modules")));
        assert!(settings.exclude_paths.contains(&PathBuf::from("target")));
    }

    #[test]
    fn test_exclude_globs_union() {
        // Test that exclude globs are properly unioned from multiple sources
        Settings::add_exclude_globs(vec!["**/*.min.js".to_string()]);
        Settings::add_exclude_globs(vec!["**/*.map".to_string()]);

        let settings = Settings::get();
        assert!(settings.exclude_globs.contains("**/*.min.js"));
        assert!(settings.exclude_globs.contains("**/*.map"));
    }

    #[test]
    fn test_exclude_no_duplicates() {
        // Test that duplicates are not added
        Settings::add_exclude_paths(vec![PathBuf::from("dist")]);
        Settings::add_exclude_paths(vec![PathBuf::from("dist")]); // Add same path twice

        let settings = Settings::get();
        let count = settings.exclude_paths.iter()
            .filter(|p| **p == PathBuf::from("dist"))
            .count();
        assert_eq!(count, 1, "Should not have duplicate paths");
    }
}
