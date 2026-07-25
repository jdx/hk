//! What each hk command does to the world.
//!
//! hk's usage spec is derived from clap, and clap has no way to express this,
//! so the classification lives here and is applied in [`crate::cli::usage`].
//!
//! The three values are defined by the usage spec:
//!
//! - `read` — only inspects state; running it twice is the same as running it
//!   once, and not running it changes nothing.
//! - `write` — creates or modifies state, but removes nothing the user cannot
//!   recreate.
//! - `destructive` — removes something the user installed or configured, where
//!   getting it back means redoing work. Deserves a confirmation prompt.
//!
//! **An unlisted command means "unknown", not "safe".** Consumers treat the
//! absence of a value as "ask", so leaving a command out is the conservative
//! choice and mislabeling one `read` is the dangerous one.

use std::collections::HashMap;

use clap_usage::usage;
use clap_usage::usage::SpecCommandEffect::{self, Destructive, Read, Write};

/// Commands whose effect is fixed, keyed by their full path under `hk`.
pub const EFFECTS: &[(&str, SpecCommandEffect)] = &[
    ("builtins", Read),
    ("cache", Read),
    // The cache is regenerated on demand, so clearing it costs only time.
    ("cache clear", Write),
    ("completion", Read),
    ("config", Read),
    ("config dump", Read),
    ("config explain", Read),
    ("config get", Read),
    ("config sources", Read),
    ("init", Write),
    ("install", Write),
    ("migrate", Read),
    ("migrate pre-commit", Write),
    ("sponsors", Read),
    // Deletes hook files from .git/hooks, which may not be exactly what was
    // there before hk was installed.
    ("uninstall", Destructive),
    ("usage", Read),
    ("util", Read),
    ("util check-added-large-files", Read),
    ("util check-byte-order-marker", Read),
    ("util check-case-conflict", Read),
    ("util check-conventional-commit", Read),
    ("util check-executables-have-shebangs", Read),
    ("util check-merge-conflict", Read),
    ("util check-symlinks", Read),
    ("util detect-private-key", Read),
    // These rewrite the files they inspect.
    ("util end-of-file-fixer", Write),
    ("util fix-byte-order-marker", Write),
    ("util fix-smart-quotes", Write),
    ("util mixed-line-ending", Write),
    ("util no-commit-to-branch", Read),
    ("util python-check-ast", Read),
    ("util python-debug-statements", Read),
    ("util trailing-whitespace", Write),
    ("validate", Read),
    ("version", Read),
];

/// Commands with no fixed effect, and why.
///
/// hk's whole job is running steps declared in `hk.pkl`, so for these the
/// effect is whatever those steps do. `check` is *intended* to be read-only and
/// `fix` to modify files, but hk cannot enforce either — a check step is an
/// arbitrary command. Claiming `read` here would be exactly the kind of
/// reassurance the field exists to avoid giving falsely.
// Only the coverage test reads this; it exists so the reason a command is
// left unclassified lives next to the decision rather than in a commit message.
#[cfg(test)]
pub const UNCLASSIFIED: &[(&str, &str)] = &[
    ("check", "runs check steps declared in hk.pkl"),
    ("fix", "runs fix steps declared in hk.pkl"),
    ("run", "runs the steps a hook declares in hk.pkl"),
    ("run commit-msg", "runs the steps this hook declares"),
    ("run post-checkout", "runs the steps this hook declares"),
    ("run post-commit", "runs the steps this hook declares"),
    ("run post-merge", "runs the steps this hook declares"),
    ("run post-rewrite", "runs the steps this hook declares"),
    ("run pre-commit", "runs the steps this hook declares"),
    ("run pre-push", "runs the steps this hook declares"),
    ("run pre-rebase", "runs the steps this hook declares"),
    (
        "run prepare-commit-msg",
        "runs the steps this hook declares",
    ),
    (
        "test",
        "runs step-defined tests, which are arbitrary commands",
    ),
];

/// Annotate every command in the spec that has a declared effect.
pub fn apply(spec: &mut usage::Spec) {
    let effects: HashMap<&str, SpecCommandEffect> = EFFECTS.iter().copied().collect();
    annotate(&mut spec.cmd, &mut vec![], &effects);
}

fn annotate(
    cmd: &mut usage::SpecCommand,
    path: &mut Vec<String>,
    effects: &HashMap<&str, SpecCommandEffect>,
) {
    for (name, sub) in cmd.subcommands.iter_mut() {
        path.push(name.clone());
        if let Some(effect) = effects.get(path.join(" ").as_str()) {
            sub.effect = Some(*effect);
        }
        annotate(sub, path, effects);
        path.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::CommandFactory;
    use std::collections::HashSet;

    /// Every command in the tree, hidden ones included: a hidden command is
    /// still runnable.
    fn all_commands() -> Vec<String> {
        let spec: usage::Spec = Cli::command().into();
        let mut out = vec![];
        collect(&spec.cmd, &mut vec![], &mut out);
        out
    }

    fn collect(cmd: &usage::SpecCommand, path: &mut Vec<String>, out: &mut Vec<String>) {
        for (name, sub) in &cmd.subcommands {
            path.push(name.clone());
            out.push(path.join(" "));
            collect(sub, path, out);
            path.pop();
        }
    }

    fn classified() -> HashSet<&'static str> {
        EFFECTS
            .iter()
            .map(|(name, _)| *name)
            .chain(UNCLASSIFIED.iter().map(|(name, _)| *name))
            .collect()
    }

    /// The tables are only worth having if they reach the spec. Everything
    /// else here checks the tables against the CLI; this checks that `apply`
    /// actually transfers them.
    #[test]
    fn apply_annotates_the_spec() {
        let mut spec: usage::Spec = Cli::command().into();
        apply(&mut spec);

        let cmd = |name: &str| {
            spec.cmd
                .subcommands
                .get(name)
                .unwrap_or_else(|| panic!("no `hk {name}`"))
        };
        assert_eq!(cmd("uninstall").effect, Some(Destructive));
        assert_eq!(cmd("builtins").effect, Some(Read));
        assert_eq!(cmd("install").effect, Some(Write));
        // Nested commands are reached too.
        assert_eq!(
            cmd("util").subcommands["trailing-whitespace"].effect,
            Some(Write)
        );
        // Anything in UNCLASSIFIED must be left unset, not defaulted.
        assert_eq!(cmd("check").effect, None);
        assert_eq!(cmd("fix").effect, None);
    }

    /// Adding a command without deciding what it does to the world is the
    /// failure mode this table exists to prevent, so make it a test failure
    /// rather than a silently missing annotation.
    #[test]
    fn every_command_is_classified() {
        let known = classified();
        let missing: Vec<String> = all_commands()
            .into_iter()
            .filter(|cmd| !known.contains(cmd.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "these commands have no entry in EFFECTS or UNCLASSIFIED \
             (src/cli/command_effects.rs) — decide whether each is read, write, \
             destructive, or genuinely unclassifiable:\n  {}",
            missing.join("\n  ")
        );
    }

    /// Catches entries left behind by a renamed or removed command.
    #[test]
    fn no_classification_refers_to_a_missing_command() {
        let present: HashSet<String> = all_commands().into_iter().collect();
        let stale: Vec<&str> = classified()
            .into_iter()
            .filter(|name| !present.contains(*name))
            .collect();
        assert!(
            stale.is_empty(),
            "these entries no longer match a command:\n  {}",
            stale.join("\n  ")
        );
    }

    #[test]
    fn classifications_are_not_duplicated() {
        let mut seen = HashSet::new();
        for name in EFFECTS
            .iter()
            .map(|(n, _)| *n)
            .chain(UNCLASSIFIED.iter().map(|(n, _)| *n))
        {
            assert!(seen.insert(name), "{name} is classified twice");
        }
    }
}
