use crate::builtins::BuiltinMeta;

/// Generate hk.pkl content based on selected builtins and hooks
pub fn generate_pkl(builtins: &[&BuiltinMeta], hooks: &[String], version: &str) -> String {
    let mut output = String::new();

    // Header with package import
    output.push_str(&format!(
        r#"amends "package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#/Builtins.pkl"

"#
    ));

    // Generate linters section (always define, even if empty)
    output.push_str("local linters = new Mapping<String, Step> {\n");
    for meta in builtins {
        output.push_str(&format!(
            "    [\"{}\"] = Builtins.{}\n",
            meta.name, meta.name
        ));
    }
    output.push_str("}\n\n");

    // Generate hooks section
    output.push_str("hooks {\n");

    for hook in hooks {
        match hook.as_str() {
            "pre-commit" => {
                output.push_str(
                    r#"    ["pre-commit"] {
        fix = true
        stash = "git"
        steps = linters
    }
"#,
                );
            }
            "pre-push" => {
                output.push_str(
                    r#"    ["pre-push"] {
        steps = linters
    }
"#,
                );
            }
            "fix" => {
                output.push_str(
                    r#"    ["fix"] {
        fix = true
        steps = linters
    }
"#,
                );
            }
            "check" => {
                output.push_str(
                    r#"    ["check"] {
        steps = linters
    }
"#,
                );
            }
            _ => {}
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

local linters = new Mapping<String, Step> {{
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

hooks {{
    ["pre-commit"] {{
        fix = true
        stash = "git"
        steps = linters
    }}
    ["fix"] {{
        fix = true
        steps = linters
    }}
    ["check"] {{
        steps = linters
    }}
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
    }

    #[test]
    fn test_generate_pkl_with_builtins() {
        let prettier = BUILTINS_META.iter().find(|b| b.name == "prettier").unwrap();
        let builtins = vec![prettier];
        let hooks = vec!["pre-commit".to_string(), "check".to_string()];
        let pkl = generate_pkl(&builtins, &hooks, "1.34.0");

        assert!(pkl.contains("Builtins.prettier"));
        assert!(pkl.contains("[\"pre-commit\"]"));
        assert!(pkl.contains("[\"check\"]"));
    }

    #[test]
    fn test_default_template() {
        let template = generate_default_template("1.34.0");
        assert!(template.contains("v1.34.0"));
        assert!(template.contains("// Add linters here"));
    }
}
