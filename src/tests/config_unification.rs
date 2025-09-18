#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use indexmap::IndexSet;

    #[test]
    fn test_exclude_union() {
        // Test that exclude patterns are properly unioned from multiple sources
        Settings::add_exclude(vec!["node_modules".to_string()]);
        Settings::add_exclude(vec!["target".to_string()]);

        let settings = Settings::get();
        assert!(settings.exclude().contains("node_modules"));
        assert!(settings.exclude().contains("target"));
    }

    #[test]
    fn test_exclude_glob_patterns() {
        // Test that glob patterns work
        Settings::add_exclude(vec!["**/*.min.js".to_string()]);
        Settings::add_exclude(vec!["**/*.map".to_string()]);

        let settings = Settings::get();
        assert!(settings.exclude().contains("**/*.min.js"));
        assert!(settings.exclude().contains("**/*.map"));
    }

    #[test]
    fn test_exclude_no_duplicates() {
        // Test that duplicates are not added
        Settings::add_exclude(vec!["dist".to_string()]);
        Settings::add_exclude(vec!["dist".to_string()]); // Add same pattern twice

        let settings = Settings::get();
        let count = settings.exclude().iter()
            .filter(|p| *p == "dist")
            .count();
        assert_eq!(count, 1, "Should not have duplicate patterns");
    }
}
