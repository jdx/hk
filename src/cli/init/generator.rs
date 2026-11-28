use crate::builtins::BuiltinMeta;

/// The imports every generated config starts with, from `root`, such as `hk:`.
/// `hk:` modules come from the hk that evaluates the config, so there is no
/// version to pin or bump.
fn header(root: &str) -> String {
    format!(
        r#"amends "{root}/Config.pkl"
import "{root}/Builtins.pkl"
// Using a coding agent? See https://hk.jdx.dev/agents
"#
    )
}

/// Generate hk.pkl content based on selected builtins and hooks
pub fn generate_pkl(builtins: &[&BuiltinMeta], hooks: &[String], root: &str) -> String {
    let mut output = String::new();

    output.push_str(&header(root));
    output.push('\n');

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

    let implicit_hooks = ["pre-commit"];
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
pub fn generate_default_template(root: &str) -> String {
    let header = header(root);
    format!(
        r#"{header}
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
        let hooks = vec![];
        let pkl = generate_pkl(&[], &hooks, "hk:");
        assert!(pkl.contains("amends"));
        assert!(pkl.contains("hooks"));
        assert!(pkl.contains("[\"pre-commit\"]"));
        assert!(pkl.contains("enabled = false"));
    }

    #[test]
    fn test_generate_pkl_with_builtins() {
        let prettier = BUILTINS_META.iter().find(|b| b.name == "prettier").unwrap();
        let builtins = vec![prettier];
        let hooks = vec!["pre-commit".to_string()];
        let pkl = generate_pkl(&builtins, &hooks, "hk:");

        assert!(pkl.contains("Builtins.prettier"));
        assert!(pkl.contains("steps = linters"));
        assert!(!pkl.contains("hooks {"));
    }

    #[test]
    fn test_generate_pkl_with_builtin_options() {
        let gitleaks = BUILTINS_META.iter().find(|b| b.name == "gitleaks").unwrap();
        let pkl = generate_pkl(&[gitleaks], &["pre-commit".to_string()], "hk:");

        assert!(pkl.contains("Builtins.gitleaks"));
    }

    #[test]
    fn test_builtin_names_are_pkl_identifiers() {
        // `Builtins.<name>` is emitted verbatim, so a `-` would not resolve.
        assert!(
            BUILTINS_META
                .iter()
                .any(|b| b.name == "editorconfig_checker")
        );
        for meta in BUILTINS_META {
            assert!(
                meta.name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{} is not a valid pkl identifier",
                meta.name
            );
        }

        let ec = BUILTINS_META
            .iter()
            .find(|b| b.name == "editorconfig_checker")
            .unwrap();
        let pkl = generate_pkl(&[ec], &["pre-commit".to_string()], "hk:");
        assert!(pkl.contains("[\"editorconfig_checker\"] = Builtins.editorconfig_checker"));
    }

    #[test]
    fn test_default_template() {
        let template = generate_default_template("hk:");
        assert!(template.starts_with("amends \"hk:/Config.pkl\"\n"));
        assert!(template.contains("// Add linters here"));
    }

    #[test]
    fn test_agent_docs_link_follows_imports() {
        let expected = "import \"hk:/Builtins.pkl\"\n// Using a coding agent? See https://hk.jdx.dev/agents\n\n";
        assert!(generate_pkl(&[], &[], "hk:").contains(expected));
        assert!(generate_default_template("hk:").contains(expected));
    }

    #[test]
    fn test_header_uses_the_given_root() {
        let root = "package://github.com/jdx/hk/releases/download/v1.34.0/hk@1.34.0#";
        let template = generate_default_template(root);
        assert!(template.starts_with(&format!(
            "amends \"{root}/Config.pkl\"\nimport \"{root}/Builtins.pkl\"\n"
        )));
    }
}
