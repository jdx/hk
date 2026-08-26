use crate::builtins::BuiltinMeta;

/// Generate hk.pkl content based on selected builtins and hooks
pub fn generate_pkl(builtins: &[&BuiltinMeta], hooks: &[String], version: &str) -> String {
    let mut output = String::new();

    // Header with package import
    output.push_str(&format!(
        r#"amends "package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Builtins.pkl"
// Using a coding agent? See https://hk.jdx.dev/agents

"#
    ));

    // Generate linters section (always define, even if empty)
    output.push_str("local linters = new Mapping {\n");
    for meta in builtins {
        output.push_str(&format!(
            "    [\"{}\"] = Builtins.{}\n",
            meta.name, meta.name
        ));
    }
    output.push_str("}\n\n");

    output.push_str("steps = linters\n");

    let implicit_hooks = ["pre-commit", "check", "fix"];
    let disabled_hooks = implicit_hooks
        .iter()
        .filter(|name| !hooks.iter().any(|hook| hook == **name))
        .collect::<Vec<_>>();
    let explicit_hooks = hooks
        .iter()
        .filter(|hook| !implicit_hooks.contains(&hook.as_str()))
        .collect::<Vec<_>>();

    if disabled_hooks.is_empty() && explicit_hooks.is_empty() {
        return output;
    }

    output.push_str("\nhooks {\n");
    for hook in disabled_hooks {
        output.push_str(&format!(
            "    [\"{hook}\"] {{\n        enabled = false\n    }}\n"
        ));
    }
    for hook in explicit_hooks {
        if hook == "pre-push" {
            output.push_str(
                r#"    ["pre-push"] {
        steps = linters
    }
"#,
            );
        }
    }

    output.push_str("}\n");

    output
}

/// Generate a simple default template for when nothing is detected
pub fn generate_default_template(version: &str) -> String {
    format!(
        r#"amends "package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Builtins.pkl"
// Using a coding agent? See https://hk.jdx.dev/agents

steps {{
    // Add linters here. Examples:
    // ["prettier"] = Builtins.prettier
    // ["eslint"] = Builtins.eslint
    // ["ruff"] = Builtins.ruff

    // Or define custom steps:
    // ["custom"] {{
    //     glob = "**/*.py"
    //     check = "mypy {{{{ files }}}}"
    // }}
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtins::BUILTINS_META;

    #[test]
    fn test_generate_pkl_empty() {
        let hooks = vec!["check".to_string()];
        let pkl = generate_pkl(&[], &hooks, "1.34.0");
        assert!(pkl.contains("amends"));
        assert!(pkl.contains("hooks"));
        assert!(pkl.contains("[\"pre-commit\"]"));
        assert!(pkl.contains("enabled = false"));
    }

    #[test]
    fn test_generate_pkl_with_builtins() {
        let prettier = BUILTINS_META.iter().find(|b| b.name == "prettier").unwrap();
        let builtins = vec![prettier];
        let hooks = vec![
            "pre-commit".to_string(),
            "check".to_string(),
            "fix".to_string(),
        ];
        let pkl = generate_pkl(&builtins, &hooks, "1.34.0");

        assert!(pkl.contains("Builtins.prettier"));
        assert!(pkl.contains("steps = linters"));
        assert!(!pkl.contains("hooks {"));
    }

    #[test]
    fn test_generate_pkl_with_builtin_options() {
        let gitleaks = BUILTINS_META.iter().find(|b| b.name == "gitleaks").unwrap();
        let pkl = generate_pkl(&[gitleaks], &["check".to_string()], "1.34.0");

        assert!(pkl.contains("Builtins.gitleaks"));
    }

    #[test]
    fn test_default_template() {
        let template = generate_default_template("1.34.0");
        assert!(template.contains("v1.34.0"));
        assert!(template.contains("// Add linters here"));
    }

    #[test]
    fn test_agent_docs_link_follows_imports() {
        let version = "test-version";
        let expected = format!(
            "import \"package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Builtins.pkl\"\n// Using a coding agent? See https://hk.jdx.dev/agents"
        );
        assert!(generate_pkl(&[], &[], version).contains(&expected));
        assert!(generate_default_template(version).contains(&expected));
    }
}
