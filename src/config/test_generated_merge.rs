#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::generated::settings::*;
    use indexmap::IndexSet;
    use std::collections::HashMap;
    use std::num::NonZero;

    #[test]
    fn test_union_merge_skip_steps() {
        let mut builder = SettingsBuilder::new();

        // Simulate values from different sources
        // Defaults
        builder.defaults.skip_steps = Some(toml::Value::Array(vec![
            toml::Value::String("default_step".to_string()),
        ]));

        // PKL config
        builder.pkl.skip_steps = Some(toml::Value::Array(vec![
            toml::Value::String("pkl_step".to_string()),
        ]));

        // Git config
        builder.git.skip_steps = Some(toml::Value::Array(vec![
            toml::Value::String("git_step".to_string()),
            toml::Value::String("pkl_step".to_string()), // Duplicate
        ]));

        // Environment
        builder.env.skip_steps = Some(toml::Value::Array(vec![
            toml::Value::String("env_step".to_string()),
        ]));

        // CLI
        builder.cli.skip_steps = Some(toml::Value::Array(vec![
            toml::Value::String("cli_step".to_string()),
            toml::Value::String("env_step".to_string()), // Duplicate
        ]));

        let (settings, _sources) = builder.build();

        // Check union merge happened correctly
        assert!(settings.skip_steps().contains("default_step"));
        assert!(settings.skip_steps().contains("pkl_step"));
        assert!(settings.skip_steps().contains("git_step"));
        assert!(settings.skip_steps().contains("env_step"));
        assert!(settings.skip_steps().contains("cli_step"));

        // Check no duplicates (IndexSet deduplicates)
        assert_eq!(settings.skip_steps().len(), 5);
    }

    #[test]
    fn test_replace_merge_fail_fast() {
        let mut builder = SettingsBuilder::new();

        // Set at different precedence levels
        builder.defaults.fail_fast = Some(toml::Value::Boolean(true));
        builder.pkl.fail_fast = Some(toml::Value::Boolean(false));
        builder.git.fail_fast = Some(toml::Value::Boolean(true));
        builder.env.fail_fast = Some(toml::Value::Boolean(false));
        builder.cli.fail_fast = Some(toml::Value::Boolean(true));

        let (settings, _sources) = builder.build();

        // CLI value should win (highest precedence)
        assert_eq!(settings.fail_fast(), true);
    }

    #[test]
    fn test_precedence_ordering() {
        let mut builder = SettingsBuilder::new();

        // Set jobs at each level with different values
        builder.defaults.jobs = Some(toml::Value::Integer(1));
        builder.pkl.jobs = Some(toml::Value::Integer(2));
        builder.git.jobs = Some(toml::Value::Integer(3));
        builder.env.jobs = Some(toml::Value::Integer(4));
        builder.cli.jobs = Some(toml::Value::Integer(5));

        let (settings, _sources) = builder.build();

        // CLI value (5) should win
        assert_eq!(settings.jobs().get(), 5);
    }

    #[test]
    fn test_precedence_with_missing_layers() {
        let mut builder = SettingsBuilder::new();

        // Only set some layers, not all
        builder.defaults.jobs = Some(toml::Value::Integer(1));
        // PKL not set
        builder.git.jobs = Some(toml::Value::Integer(3));
        // env not set
        // CLI not set

        let (settings, _sources) = builder.build();

        // Git value (3) should win as highest set precedence
        assert_eq!(settings.jobs().get(), 3);
    }

    #[test]
    fn test_source_tracking() {
        let mut builder = SettingsBuilder::new();

        // Simulate adding from env source
        builder.env.jobs = Some(toml::Value::Integer(8));
        builder.sources.insert("jobs".to_string(), "env:HK_JOBS".to_string());

        let (_settings, sources) = builder.build();

        // Check source was tracked
        assert_eq!(sources.sources.get("jobs"), Some(&"env:HK_JOBS".to_string()));
    }

    #[test]
    fn test_exclude_union_merge() {
        let mut builder = SettingsBuilder::new();

        // Set exclude patterns at different levels
        builder.defaults.exclude = Some(toml::Value::Array(vec![
            toml::Value::String("*.tmp".to_string()),
        ]));

        builder.git.exclude = Some(toml::Value::Array(vec![
            toml::Value::String("*.log".to_string()),
            toml::Value::String("*.tmp".to_string()), // Duplicate
        ]));

        builder.env.exclude = Some(toml::Value::Array(vec![
            toml::Value::String("*.bak".to_string()),
        ]));

        let (settings, _sources) = builder.build();

        // All patterns should be included (union)
        assert!(settings.exclude().contains("*.tmp"));
        assert!(settings.exclude().contains("*.log"));
        assert!(settings.exclude().contains("*.bak"));

        // No duplicates
        assert_eq!(settings.exclude().len(), 3);
    }

    #[test]
    fn test_display_skip_reasons_default() {
        let builder = SettingsBuilder::new();
        // Don't set display_skip_reasons anywhere

        let (settings, _sources) = builder.build();

        // Should get the special default
        assert!(settings.display_skip_reasons().contains("profile-not-enabled"));
    }

    #[test]
    fn test_list_replace_merge() {
        let mut builder = SettingsBuilder::new();

        // warnings uses replace merge, not union
        builder.defaults.warnings = Some(toml::Value::Array(vec![
            toml::Value::String("warning1".to_string()),
        ]));

        builder.git.warnings = Some(toml::Value::Array(vec![
            toml::Value::String("warning2".to_string()),
            toml::Value::String("warning3".to_string()),
        ]));

        builder.env.warnings = Some(toml::Value::Array(vec![
            toml::Value::String("warning4".to_string()),
        ]));

        let (settings, _sources) = builder.build();

        // Only env value should be present (replace, not union)
        assert!(settings.warnings().contains("warning4"));
        assert!(!settings.warnings().contains("warning1"));
        assert!(!settings.warnings().contains("warning2"));
        assert!(!settings.warnings().contains("warning3"));
        assert_eq!(settings.warnings().len(), 1);
    }
}
