use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::Result;
use eyre::{WrapErr, bail};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Deserializer};

/// Migrate from pre-commit (or prek) to hk
///
/// Reads a .pre-commit-config.yaml and writes an hk.pkl that runs the same
/// hooks at the same git stages:
///
/// - Hooks from well-known repositories become hk builtins, for example
///   `ruff` from astral-sh/ruff-pre-commit becomes `Builtins.ruff`.
/// - Local `system`, `script`, and `fail` hooks become native hk steps with
///   the same command and file filters.
/// - Every other hook keeps running through prek or pre-commit, which reads
///   the original config. This includes hooks whose `args`,
///   `additional_dependencies`, or filters a builtin cannot reproduce.
///   Each of these steps has a comment explaining why it was not converted.
///
/// `manual` hooks run only with `hk check` and `hk fix`.
#[derive(Debug, usage_rs::Args)]
#[usage(effect = "write")]
pub struct PreCommit {
    /// Path to .pre-commit-config.yaml
    #[usage(short, long, default = ".pre-commit-config.yaml")]
    config: PathBuf,
    /// Overwrite existing hk.pkl file
    #[usage(short, long)]
    force: bool,
    /// Output path for hk.pkl
    #[usage(short, long, default = "hk.pkl")]
    output: PathBuf,
    /// Tool that runs hooks hk cannot convert.
    /// Defaults to prek when it is on PATH, otherwise pre-commit.
    #[usage(long, choices("prek", "pre-commit"), verbatim_doc_comment)]
    runner: Option<String>,
    /// Root path for hk pkl files (e.g. "pkl" for a local checkout, or a package URL prefix).
    /// If set, the generated config uses {root}/Config.pkl and {root}/Builtins.pkl
    #[usage(long)]
    hk_pkl_root: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PreCommitConfig {
    repos: Vec<Repo>,
    #[serde(default)]
    files: Option<String>,
    #[serde(default)]
    exclude: Option<String>,
    #[serde(default)]
    default_stages: Vec<String>,
    #[serde(default)]
    default_install_hook_types: Vec<String>,
    #[serde(default)]
    fail_fast: bool,
}

#[derive(Debug, Deserialize)]
struct Repo {
    repo: String,
    #[serde(default)]
    hooks: Vec<Hook>,
}

#[derive(Debug, Deserialize)]
struct Hook {
    id: String,
    entry: Option<String>,
    language: Option<String>,
    files: Option<String>,
    exclude: Option<String>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default)]
    types_or: Vec<String>,
    #[serde(default)]
    exclude_types: Vec<String>,
    #[serde(default, deserialize_with = "scalar_list")]
    args: Vec<String>,
    #[serde(default)]
    additional_dependencies: Vec<String>,
    #[serde(default)]
    stages: Vec<String>,
    #[serde(default)]
    always_run: bool,
    pass_filenames: Option<bool>,
    language_version: Option<String>,
}

/// Accept `args: [--max-line-length, 100]`, where YAML parses 100 as a number.
fn scalar_list<'de, D: Deserializer<'de>>(de: D) -> std::result::Result<Vec<String>, D::Error> {
    let values = Vec::<serde_yaml::Value>::deserialize(de)?;
    values
        .into_iter()
        .map(|v| match v {
            serde_yaml::Value::String(s) => Ok(s),
            serde_yaml::Value::Number(n) => Ok(n.to_string()),
            serde_yaml::Value::Bool(b) => Ok(b.to_string()),
            other => Err(serde::de::Error::custom(format!(
                "expected a string argument, got {other:?}"
            ))),
        })
        .collect()
}

/// Hooks with an equivalent hk builtin, keyed by `owner/repo` and hook id.
const BUILTINS: &[(&str, &[(&str, &str)])] = &[
    (
        "pre-commit/pre-commit-hooks",
        &[
            ("check-added-large-files", "check_added_large_files"),
            ("check-ast", "python_check_ast"),
            ("check-byte-order-marker", "byte_order_marker"),
            ("check-case-conflict", "check_case_conflict"),
            (
                "check-executables-have-shebangs",
                "check_executables_have_shebangs",
            ),
            ("check-merge-conflict", "check_merge_conflict"),
            (
                "check-shebang-scripts-are-executable",
                "check_shebang_scripts_are_executable",
            ),
            ("check-symlinks", "check_symlinks"),
            ("debug-statements", "python_debug_statements"),
            ("destroyed-symlinks", "destroyed_symlinks"),
            ("detect-private-key", "detect_private_key"),
            ("end-of-file-fixer", "newlines"),
            ("fix-byte-order-marker", "byte_order_marker"),
            ("forbid-submodules", "forbid_submodules"),
            ("mixed-line-ending", "mixed_line_ending"),
            ("no-commit-to-branch", "no_commit_to_branch"),
            ("trailing-whitespace", "trailing_whitespace"),
        ],
    ),
    ("adrienverge/yamllint", &[("yamllint", "yamllint")]),
    (
        "antonbabenko/pre-commit-terraform",
        &[
            ("terraform_fmt", "terraform"),
            ("terraform_tflint", "tf_lint"),
            ("terraform_validate", "terraform_validate"),
        ],
    ),
    (
        "astral-sh/ruff-pre-commit",
        &[
            ("ruff", "ruff"),
            ("ruff-check", "ruff"),
            ("ruff-format", "ruff_format"),
        ],
    ),
    ("biomejs/pre-commit", &[("biome-check", "biome")]),
    (
        "comppwa/taplo-pre-commit",
        &[("taplo-format", "taplo_format"), ("taplo-lint", "taplo")],
    ),
    (
        "compilerla/conventional-pre-commit",
        &[("conventional-pre-commit", "check_conventional_commit")],
    ),
    ("crate-ci/typos", &[("typos", "typos")]),
    (
        "dnephin/pre-commit-golang",
        &[
            ("go-fmt", "go_fmt"),
            ("go-imports", "go_imports"),
            ("go-vet", "go_vet"),
            ("golangci-lint", "golangci_lint"),
        ],
    ),
    (
        "doublify/pre-commit-rust",
        &[
            ("cargo-check", "cargo_check"),
            ("clippy", "cargo_clippy"),
            ("fmt", "cargo_fmt"),
        ],
    ),
    (
        "editorconfig-checker/editorconfig-checker.python",
        &[("editorconfig-checker", "editorconfig_checker")],
    ),
    ("gitleaks/gitleaks", &[("gitleaks", "gitleaks")]),
    (
        "golangci/golangci-lint",
        &[("golangci-lint", "golangci_lint")],
    ),
    ("google/yamlfmt", &[("yamlfmt", "yamlfmt")]),
    (
        "hadolint/hadolint",
        &[("hadolint", "hadolint"), ("hadolint-docker", "hadolint")],
    ),
    (
        "igorshubovych/markdownlint-cli",
        &[("markdownlint", "markdown_lint")],
    ),
    (
        "johnnymorganz/stylua",
        &[
            ("stylua", "stylua"),
            ("stylua-github", "stylua"),
            ("stylua-system", "stylua"),
        ],
    ),
    (
        "koalaman/shellcheck-precommit",
        &[("shellcheck", "shellcheck")],
    ),
    ("lycheeverse/lychee", &[("lychee", "lychee")]),
    ("pre-commit/mirrors-eslint", &[("eslint", "eslint")]),
    ("pre-commit/mirrors-mypy", &[("mypy", "mypy")]),
    ("pre-commit/mirrors-prettier", &[("prettier", "prettier")]),
    ("psf/black", &[("black", "black")]),
    ("psf/black-pre-commit-mirror", &[("black", "black")]),
    ("pycqa/flake8", &[("flake8", "flake8")]),
    ("pycqa/isort", &[("isort", "isort")]),
    ("rbubley/mirrors-prettier", &[("prettier", "prettier")]),
    ("rhysd/actionlint", &[("actionlint", "actionlint")]),
    ("rubocop/rubocop", &[("rubocop", "rubocop")]),
    ("scop/pre-commit-shfmt", &[("shfmt", "shfmt")]),
    (
        "shellcheck-py/shellcheck-py",
        &[("shellcheck", "shellcheck")],
    ),
    (
        "sqlfluff/sqlfluff",
        &[
            ("sqlfluff-fix", "sql_fluff"),
            ("sqlfluff-lint", "sql_fluff"),
        ],
    ),
    (
        "tamasfe/taplo",
        &[("taplo-format", "taplo_format"), ("taplo-lint", "taplo")],
    ),
    ("woodruffw/zizmor-pre-commit", &[("zizmor", "zizmor")]),
    ("zizmorcore/zizmor-pre-commit", &[("zizmor", "zizmor")]),
];

/// Builtins that define their own `exclude`, which a migrated regex would replace.
const BUILTINS_WITH_EXCLUDE: &[&str] = &["rubocop", "terraform_validate", "tf_lint"];

/// Args that only enable behavior hk already provides through `hk fix`.
fn redundant_args(builtin: &str) -> &'static [&'static str] {
    match builtin {
        "ruff" => &["--fix", "--exit-non-zero-on-fix"],
        "eslint" | "markdown_lint" => &["--fix"],
        "biome" => &["--write"],
        "mixed_line_ending" => &["--fix=auto"],
        _ => &[],
    }
}

/// Stages declared by upstream hook manifests for hooks that don't run at pre-commit.
fn manifest_stage(id: &str) -> Option<&'static str> {
    match id {
        "commitizen" | "commitlint" | "conventional-pre-commit" | "gitlint" => Some("commit-msg"),
        "commitizen-branch" => Some("pre-push"),
        _ => None,
    }
}

/// Git hooks hk can run, plus `manual` (check and fix only).
const STAGES: &[&str] = &[
    "pre-commit",
    "pre-push",
    "commit-msg",
    "prepare-commit-msg",
    "post-checkout",
    "post-commit",
    "post-merge",
    "post-rewrite",
    "pre-rebase",
    "manual",
];

/// Git hooks where pre-commit passes changed files to hooks.
const FILE_STAGES: &[&str] = &["pre-commit", "pre-merge-commit", "pre-push"];

fn normalize_stage(stage: &str) -> &str {
    match stage {
        "commit" => "pre-commit",
        "push" => "pre-push",
        "merge-commit" => "pre-merge-commit",
        other => other,
    }
}

/// Map a pre-commit (identify) file type tag to an hk file type.
fn hk_type(tag: &str) -> Option<&'static str> {
    Some(match tag {
        "ts" | "typescript" => "typescript",
        "python" => "python",
        "pyi" => "pyi",
        "javascript" => "javascript",
        "jsx" => "jsx",
        "tsx" => "tsx",
        "rust" => "rust",
        "go" => "go",
        "ruby" => "ruby",
        "php" => "php",
        "java" => "java",
        "kotlin" => "kotlin",
        "swift" => "swift",
        "c" => "c",
        "c++" => "c++",
        "lua" => "lua",
        "shell" => "shell",
        "bash" => "bash",
        "zsh" => "zsh",
        "fish" => "fish",
        "sh" => "sh",
        "json" => "json",
        "yaml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "csv" => "csv",
        "html" => "html",
        "markdown" => "markdown",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        "svelte" => "svelte",
        "vue" => "vue",
        "astro" => "astro",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "pkl" => "pkl",
        "text" => "text",
        "binary" => "binary",
        "executable" => "executable",
        "symlink" => "symlink",
        "image" => "image",
        "png" => "png",
        "jpeg" => "jpeg",
        "gif" => "gif",
        "svg" => "svg",
        "webp" => "webp",
        "zip" => "zip",
        "tar" => "tar",
        "gzip" => "gzip",
        _ => return None,
    })
}

#[derive(Debug)]
enum Action {
    Builtin(&'static str),
    Command {
        glob: Option<String>,
        types: Vec<String>,
        check: String,
    },
    /// Run the hook through prek/pre-commit; the string explains why.
    Delegate(String),
}

#[derive(Debug, Clone)]
enum Exclude {
    /// Only the top-level `exclude`, rendered once as a shared local.
    Global,
    Regex(String),
}

#[derive(Debug)]
struct Step {
    hook_id: String,
    action: Action,
    exclude: Option<Exclude>,
}

#[derive(Debug, Default)]
struct Migration {
    /// Steps grouped by pre-commit stage, keyed by hk step name.
    stages: IndexMap<String, IndexMap<String, Step>>,
    global_exclude: Option<String>,
    fail_fast: bool,
    warnings: Vec<String>,
}

impl Migration {
    fn count(&self, pred: fn(&Action) -> bool) -> usize {
        self.stages
            .values()
            .flat_map(|steps| steps.values())
            .filter(|s| pred(&s.action))
            .count()
    }

    fn uses_global_exclude(&self) -> bool {
        self.stages
            .values()
            .flat_map(|steps| steps.values())
            .any(|s| matches!(s.exclude, Some(Exclude::Global)))
    }

    fn delegated_ids(&self) -> IndexSet<&str> {
        self.stages
            .values()
            .flat_map(|steps| steps.values())
            .filter(|s| matches!(s.action, Action::Delegate(_)))
            .map(|s| s.hook_id.as_str())
            .collect()
    }
}

impl PreCommit {
    pub async fn run(&self) -> Result<()> {
        if self.output.exists() && !self.force {
            bail!(
                "{} already exists, use --force to overwrite",
                self.output.display()
            );
        }
        if !self.config.exists() {
            bail!("{} does not exist", self.config.display());
        }

        let content = xx::file::read_to_string(&self.config)?;
        let config: PreCommitConfig = serde_yaml::from_str(&content)
            .wrap_err_with(|| format!("failed to parse {}", self.config.display()))?;

        let runner = self.runner.clone().unwrap_or_else(|| {
            if xx::file::which("prek").is_none() && xx::file::which("pre-commit").is_some() {
                "pre-commit".to_string()
            } else {
                "prek".to_string()
            }
        });

        let migration = convert(&config);
        for warning in &migration.warnings {
            warn!("{warning}");
        }
        let pkl = self.render(&migration, &runner);
        xx::file::write(&self.output, pkl)?;

        info!(
            "Migrated {} to {}: {} builtins, {} commands, {} run through {runner}",
            self.config.display(),
            self.output.display(),
            migration.count(|a| matches!(a, Action::Builtin(_))),
            migration.count(|a| matches!(a, Action::Command { .. })),
            migration.count(|a| matches!(a, Action::Delegate(_))),
        );
        let delegated = migration.delegated_ids();
        if !delegated.is_empty() {
            info!(
                "Keep {} and {runner} installed for: {}",
                self.config.display(),
                delegated.into_iter().collect::<Vec<_>>().join(", ")
            );
        }
        info!("Next steps:");
        info!("  1. Review {}", self.output.display());
        info!("  2. Remove pre-commit's git hooks with `{runner} uninstall`");
        info!("  3. Run `hk install`, then `hk check --all`");
        Ok(())
    }

    fn render(&self, migration: &Migration, runner: &str) -> String {
        let version = env!("CARGO_PKG_VERSION");
        let root = self.hk_pkl_root.clone().unwrap_or_else(|| {
            format!("package://github.com/jdx/hk/releases/download/v{version}/hk@{version}#")
        });
        let root = root.trim_end_matches('/');

        let mut out = String::new();
        writeln!(out, "amends \"{root}/Config.pkl\"").unwrap();
        writeln!(out, "import \"{root}/Builtins.pkl\"").unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "// Migrated from {} by `hk migrate pre-commit`.",
            self.config.display()
        )
        .unwrap();

        if !migration.delegated_ids().is_empty() {
            let mut run = format!("{runner} run");
            if self.config != Path::new(".pre-commit-config.yaml") {
                write!(
                    run,
                    " --config {}",
                    sh_quote(&self.config.to_string_lossy())
                )
                .unwrap();
            }
            writeln!(
                out,
                "\n// Steps using precommit() still run through {runner}, which reads {}.",
                self.config.display()
            )
            .unwrap();
            writeln!(
                out,
                "// Replace them with builtins or commands, then delete that file."
            )
            .unwrap();
            writeln!(
                out,
                "local function precommit(hook: String, stage: String): Step = new {{"
            )
            .unwrap();
            writeln!(
                out,
                "  check = \"{run} --hook-stage \\(stage) \\(hook) \" + (if (stage == \"commit-msg\" || stage == \"prepare-commit-msg\")"
            )
            .unwrap();
            writeln!(out, "    \"--commit-msg-filename {{{{commit_msg_file}}}}\"").unwrap();
            writeln!(out, "  else").unwrap();
            writeln!(out, "    \"--files {{{{files}}}}\")").unwrap();
            writeln!(out, "  // pre-commit hooks may modify files in either mode").unwrap();
            writeln!(out, "  fix = check").unwrap();
            writeln!(out, "  check_first = false").unwrap();
            writeln!(out, "}}").unwrap();
        }

        if !migration.fail_fast {
            writeln!(
                out,
                "\n// Like pre-commit, report every failing step instead of stopping at the first"
            )
            .unwrap();
            writeln!(out, "fail_fast = false").unwrap();
        }

        if let Some(exclude) = &migration.global_exclude
            && migration.uses_global_exclude()
        {
            writeln!(out, "\n// pre-commit's top-level `exclude`").unwrap();
            writeln!(out, "local excluded = {}", pkl_regex(exclude)).unwrap();
        }

        let manual = migration.stages.get("manual");
        if let Some(steps) = manual {
            writeln!(out, "\nlocal manual_steps = new Mapping<String, Step> {{").unwrap();
            render_steps(&mut out, steps, "manual", 1);
            writeln!(out, "}}").unwrap();
        }

        if let Some(steps) = migration.stages.get("pre-commit") {
            writeln!(out, "\nsteps {{").unwrap();
            render_steps(&mut out, steps, "pre-commit", 1);
            writeln!(out, "}}").unwrap();
        }

        let hooks: Vec<_> = migration
            .stages
            .iter()
            .filter(|(stage, _)| *stage != "pre-commit" && *stage != "manual")
            .collect();
        if !hooks.is_empty() || manual.is_some() {
            writeln!(out, "\nhooks {{").unwrap();
            for (stage, steps) in hooks {
                writeln!(out, "  [\"{stage}\"] {{").unwrap();
                writeln!(out, "    steps {{").unwrap();
                render_steps(&mut out, steps, stage, 3);
                writeln!(out, "    }}").unwrap();
                writeln!(out, "  }}").unwrap();
            }
            if manual.is_some() {
                writeln!(out, "  // pre-commit `manual` hooks").unwrap();
                writeln!(out, "  [\"check\"] {{ steps {{ ...manual_steps }} }}").unwrap();
                writeln!(out, "  [\"fix\"] {{ steps {{ ...manual_steps }} }}").unwrap();
            }
            writeln!(out, "}}").unwrap();
        }
        out
    }
}

fn render_steps(out: &mut String, steps: &IndexMap<String, Step>, stage: &str, depth: usize) {
    let pad = "  ".repeat(depth);
    for (name, step) in steps {
        let key = format!("{pad}[{}]", pkl_str(name));
        let exclude = step.exclude.as_ref().map(|e| match e {
            Exclude::Global => format!("{pad}  exclude = excluded\n"),
            Exclude::Regex(re) => format!("{pad}  exclude = {}\n", pkl_regex(re)),
        });
        match &step.action {
            Action::Builtin(builtin) => match exclude {
                None => writeln!(out, "{key} = Builtins.{builtin}").unwrap(),
                Some(exclude) => {
                    write!(out, "{key} = (Builtins.{builtin}) {{\n{exclude}{pad}}}\n").unwrap()
                }
            },
            Action::Command { glob, types, check } => {
                writeln!(out, "{key} {{").unwrap();
                if let Some(glob) = glob {
                    writeln!(out, "{pad}  glob = {}", pkl_regex(glob)).unwrap();
                }
                if !types.is_empty() {
                    let types: Vec<_> = types.iter().map(|t| pkl_str(t)).collect();
                    writeln!(out, "{pad}  types = List({})", types.join(", ")).unwrap();
                }
                if let Some(exclude) = exclude {
                    out.push_str(&exclude);
                }
                writeln!(out, "{pad}  check = {}", pkl_str(check)).unwrap();
                writeln!(out, "{pad}}}").unwrap();
            }
            Action::Delegate(reason) => {
                writeln!(out, "{pad}// {reason}").unwrap();
                writeln!(
                    out,
                    "{key} = precommit({}, {})",
                    pkl_str(&sh_quote(&step.hook_id)),
                    pkl_str(stage)
                )
                .unwrap();
            }
        }
    }
}

fn convert(config: &PreCommitConfig) -> Migration {
    let mut migration = Migration::default();
    let global_exclude = non_empty(&config.exclude, "^$");
    migration.global_exclude = global_exclude.map(str::to_string);
    migration.fail_fast = config.fail_fast;
    let global_files = non_empty(&config.files, "");

    let installed: IndexSet<&str> = if config.default_install_hook_types.is_empty() {
        IndexSet::from(["pre-commit"])
    } else {
        config
            .default_install_hook_types
            .iter()
            .map(|s| normalize_stage(s))
            .collect()
    };
    let mut unsupported: IndexMap<String, IndexSet<String>> = IndexMap::new();

    // Collect steps per stage in config order, then name them.
    let mut pending: IndexMap<String, Vec<Step>> = IndexMap::new();
    for repo in &config.repos {
        if repo.repo == "meta" {
            continue;
        }
        for hook in &repo.hooks {
            // Stages the hook asks for explicitly always count. Defaults only
            // apply to installed git hooks that run on changed files; at other
            // stages pre-commit gives file hooks nothing to check.
            let mut stages: IndexSet<&str> = if !hook.stages.is_empty() {
                hook.stages.iter().map(|s| normalize_stage(s)).collect()
            } else if let Some(stage) = manifest_stage(&hook.id) {
                IndexSet::from([stage])
            } else {
                let defaults: Vec<&str> = if config.default_stages.is_empty() {
                    installed.iter().copied().collect()
                } else {
                    config
                        .default_stages
                        .iter()
                        .map(|s| normalize_stage(s))
                        .collect()
                };
                defaults
                    .into_iter()
                    .filter(|s| installed.contains(s) && FILE_STAGES.contains(s))
                    .collect()
            };
            stages.retain(|stage| {
                let supported = STAGES.contains(stage);
                if !supported {
                    unsupported
                        .entry(stage.to_string())
                        .or_default()
                        .insert(hook.id.clone());
                }
                supported
            });

            let (action, exclude) = convert_hook(repo, hook, global_exclude, global_files);
            for stage in stages {
                // Message hooks receive the commit message file, not staged files.
                let message_hook = matches!(stage, "commit-msg" | "prepare-commit-msg");
                let action = match &action {
                    Action::Builtin(b) => Action::Builtin(b),
                    Action::Command { check, .. } if message_hook => Action::Command {
                        glob: None,
                        types: vec![],
                        check: check.replace("{{files}}", "{{commit_msg_file}}"),
                    },
                    Action::Command { glob, types, check } => Action::Command {
                        glob: glob.clone(),
                        types: types.clone(),
                        check: check.clone(),
                    },
                    Action::Delegate(r) => Action::Delegate(r.clone()),
                };
                pending.entry(stage.to_string()).or_default().push(Step {
                    hook_id: hook.id.clone(),
                    action,
                    exclude: if message_hook { None } else { exclude.clone() },
                });
            }
        }
    }

    for (stage, ids) in unsupported {
        migration.warnings.push(format!(
            "hk has no {stage} hook, so these hooks will not run at that stage: {}",
            ids.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    for (stage, steps) in pending {
        // `pre-commit run <id>` runs every hook with that id, so a delegated
        // id takes over all of its entries in the stage.
        let mut delegated: IndexMap<String, String> = IndexMap::new();
        for step in &steps {
            if let Action::Delegate(reason) = &step.action {
                delegated
                    .entry(step.hook_id.clone())
                    .or_insert_with(|| reason.clone());
            }
        }
        let named = migration.stages.entry(stage).or_default();
        for mut step in steps {
            if let Some(reason) = delegated.get(&step.hook_id) {
                if named.contains_key(&step.hook_id) {
                    continue;
                }
                step.action = Action::Delegate(reason.clone());
                step.exclude = None;
                named.insert(step.hook_id.clone(), step);
                continue;
            }
            let mut name = step.hook_id.clone();
            let mut n = 2;
            while named.contains_key(&name) {
                name = format!("{}-{n}", step.hook_id);
                n += 1;
            }
            named.insert(name, step);
        }
    }
    migration
}

/// Treat a missing value or pre-commit's default as unset.
fn non_empty<'a>(value: &'a Option<String>, default: &str) -> Option<&'a str> {
    value
        .as_deref()
        .map(str::trim_end)
        .filter(|v| !v.is_empty() && *v != default)
}

/// Decide how to run one hook, returning the action and its exclude regex.
fn convert_hook(
    repo: &Repo,
    hook: &Hook,
    global_exclude: Option<&str>,
    global_files: Option<&str>,
) -> (Action, Option<Exclude>) {
    let delegate = |reason: String| (Action::Delegate(reason), None);

    if global_files.is_some() {
        return delegate("the top-level `files` filter has no hk equivalent".into());
    }
    let hook_exclude = non_empty(&hook.exclude, "^$");
    let exclude = match (hook_exclude, global_exclude) {
        (Some(a), Some(b)) => Some(Exclude::Regex(format!("{}|{}", group(a), group(b)))),
        (Some(a), None) => Some(Exclude::Regex(a.to_string())),
        (None, Some(_)) => Some(Exclude::Global),
        (None, None) => None,
    };
    let exclude_re = match &exclude {
        Some(Exclude::Regex(re)) => Some(re.as_str()),
        Some(Exclude::Global) => global_exclude,
        None => None,
    };
    if let Some(exclude) = exclude_re
        && regex::Regex::new(exclude).is_err()
    {
        return delegate(
            "its `exclude` regex uses syntax hk's regex engine does not support".into(),
        );
    }

    if repo.repo == "local" {
        let language = hook.language.as_deref().unwrap_or("system");
        if !matches!(
            language,
            "system" | "script" | "unsupported" | "unsupported_script" | "fail"
        ) {
            return delegate(format!(
                "pre-commit sets up a {language} environment for this hook"
            ));
        }
        let Some(entry) = hook.entry.as_deref() else {
            return delegate("local hook has no `entry`".into());
        };
        if !hook.exclude_types.is_empty() {
            return delegate("`exclude_types` has no hk equivalent".into());
        }
        let mut types = match translate_types(&hook.types, &hook.types_or) {
            Ok(types) => types,
            Err(reason) => return delegate(reason),
        };
        let mut glob = non_empty(&hook.files, "").map(str::to_string);
        if let Some(glob) = &glob
            && regex::Regex::new(glob).is_err()
        {
            return delegate(
                "its `files` regex uses syntax hk's regex engine does not support".into(),
            );
        }
        let pass_filenames = hook.pass_filenames.unwrap_or(true);
        if hook.always_run && pass_filenames {
            // hk skips a step when no files match; pre-commit runs it anyway
            return delegate("`always_run` with filenames has no hk equivalent".into());
        }
        if hook.always_run {
            glob = None;
            types.clear();
        }
        let check = if language == "fail" {
            format!(
                "printf '%s\\n' {} {{{{files}}}} >&2; exit 1",
                sh_quote(entry.trim())
            )
        } else {
            let mut cmd = entry.trim().to_string();
            for arg in &hook.args {
                cmd.push(' ');
                cmd.push_str(&sh_quote(arg));
            }
            if pass_filenames {
                cmd.push_str(" {{files}}");
            }
            cmd
        };
        return (Action::Command { glob, types, check }, exclude);
    }

    let Some(builtin) = find_builtin(&repo.repo, &hook.id) else {
        return delegate(format!("no hk builtin for {} from {}", hook.id, repo.repo));
    };
    let redundant = redundant_args(builtin);
    let args: Vec<&str> = hook
        .args
        .iter()
        .map(String::as_str)
        .filter(|a| !redundant.contains(a))
        .collect();
    let mut overrides = Vec::new();
    if !args.is_empty() {
        overrides.push(format!("args ({})", args.join(" ")));
    }
    for (set, field) in [
        (hook.files.is_some(), "files"),
        (hook.entry.is_some(), "entry"),
        (hook.language.is_some(), "language"),
        (!hook.types.is_empty(), "types"),
        (!hook.types_or.is_empty(), "types_or"),
        (!hook.exclude_types.is_empty(), "exclude_types"),
        (
            !hook.additional_dependencies.is_empty(),
            "additional_dependencies",
        ),
        (hook.language_version.is_some(), "language_version"),
        (hook.pass_filenames.is_some(), "pass_filenames"),
        (hook.always_run, "always_run"),
    ] {
        if set {
            overrides.push(format!("`{field}`"));
        }
    }
    if exclude.is_some() && BUILTINS_WITH_EXCLUDE.contains(&builtin) {
        overrides.push("`exclude`".into());
    }
    if !overrides.is_empty() {
        return delegate(format!(
            "Builtins.{builtin} does not support this hook's {}",
            overrides.join(", ")
        ));
    }
    (Action::Builtin(builtin), exclude)
}

fn find_builtin(repo_url: &str, id: &str) -> Option<&'static str> {
    let url = repo_url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .to_lowercase();
    let mut parts = url.rsplit(['/', ':']);
    let name = parts.next()?;
    let owner = parts.next()?;
    let slug = format!("{owner}/{name}");
    let (_, hooks) = BUILTINS.iter().find(|(repo, _)| *repo == slug)?;
    hooks
        .iter()
        .find(|(hook, _)| *hook == id)
        .map(|(_, builtin)| *builtin)
}

/// Convert pre-commit `types` (all must match) and `types_or` (any must
/// match) to hk `types`, which match if any type matches.
fn translate_types(
    types: &[String],
    types_or: &[String],
) -> std::result::Result<Vec<String>, String> {
    let all: Vec<&str> = types
        .iter()
        .map(String::as_str)
        .filter(|t| *t != "file")
        .collect();
    let any: Vec<&str> = if types_or.iter().any(|t| t == "file") {
        vec![]
    } else {
        types_or.iter().map(String::as_str).collect()
    };
    let wanted: Vec<&str> = match (all.as_slice(), any.is_empty()) {
        ([], _) => any,
        ([t], true) => vec![*t],
        (["text"], false) => any,
        ([a, b], true) if *a == "text" || *b == "text" => {
            vec![if *a == "text" { *b } else { *a }]
        }
        _ => {
            return Err(format!(
                "`types: [{}]` combined with `types_or` has no hk equivalent",
                types.join(", ")
            ));
        }
    };
    wanted
        .into_iter()
        .map(|t| {
            hk_type(t)
                .map(str::to_string)
                .ok_or_else(|| format!("hk has no `{t}` file type"))
        })
        .collect()
}

/// Wrap a regex so it can be joined with `|`. Verbose patterns need a
/// newline so a trailing comment does not swallow the closing paren.
fn group(re: &str) -> String {
    if re.contains('\n') {
        format!("(?:{re}\n)")
    } else {
        format!("(?:{re})")
    }
}

fn pkl_regex(re: &str) -> String {
    format!("Regex({})", pkl_str(re))
}

/// Format a Pkl string literal, using raw (`#"..."#`) delimiters when the
/// value contains backslashes or quotes so regexes read as written.
fn pkl_str(value: &str) -> String {
    let value = value.trim_end_matches('\n');
    let multiline = value.contains('\n');
    let pounds = if value.contains('\\') || value.contains('"') {
        (1..)
            .map(|n| "#".repeat(n))
            .find(|p| !value.contains(&format!("\"{p}")) && !value.contains(&format!("\\{p}")))
            .unwrap()
    } else {
        String::new()
    };
    if multiline {
        format!("{pounds}\"\"\"\n{value}\n\"\"\"{pounds}")
    } else {
        format!("{pounds}\"{value}\"{pounds}")
    }
}

/// Quote a word for sh only when needed.
fn sh_quote(word: &str) -> String {
    if !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./=:,+@%".contains(c))
    {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrate(yaml: &str) -> Migration {
        convert(&serde_yaml::from_str(yaml).unwrap())
    }

    fn step<'a>(m: &'a Migration, stage: &str, name: &str) -> &'a Step {
        &m.stages[stage][name]
    }

    #[test]
    fn known_hooks_become_builtins() {
        let m = migrate(
            r#"
repos:
- repo: https://github.com/astral-sh/ruff-pre-commit.git
  rev: v0.6.0
  hooks:
  - id: ruff
    args: [--fix]
  - id: ruff-format
- repo: git@github.com:pre-commit/pre-commit-hooks
  rev: v5.0.0
  hooks:
  - id: trailing-whitespace
    exclude: ^docs/
"#,
        );
        assert!(matches!(
            step(&m, "pre-commit", "ruff").action,
            Action::Builtin("ruff")
        ));
        assert!(matches!(
            step(&m, "pre-commit", "ruff-format").action,
            Action::Builtin("ruff_format")
        ));
        let ws = step(&m, "pre-commit", "trailing-whitespace");
        assert!(matches!(ws.action, Action::Builtin("trailing_whitespace")));
        assert!(matches!(&ws.exclude, Some(Exclude::Regex(re)) if re == "^docs/"));
    }

    #[test]
    fn hook_ids_only_match_their_repo() {
        let m = migrate(
            r#"
repos:
- repo: https://github.com/example/hooks
  rev: v1
  hooks:
  - id: black
"#,
        );
        assert!(matches!(
            step(&m, "pre-commit", "black").action,
            Action::Delegate(_)
        ));
    }

    #[test]
    fn builtin_with_args_is_delegated() {
        let m = migrate(
            r#"
repos:
- repo: https://github.com/psf/black
  rev: 24.1.0
  hooks:
  - id: black
    args: [--line-length, 100]
"#,
        );
        let Action::Delegate(reason) = &step(&m, "pre-commit", "black").action else {
            panic!("expected delegation");
        };
        assert!(reason.contains("--line-length 100"), "{reason}");
    }

    #[test]
    fn local_system_hook_becomes_command() {
        let m = migrate(
            r#"
exclude: ^vendor/
repos:
- repo: local
  hooks:
  - id: pytest
    entry: uv run pytest
    language: system
    types: [python]
    args: ["-k", "not slow"]
    stages: [pre-push]
"#,
        );
        let s = step(&m, "pre-push", "pytest");
        let Action::Command { glob, types, check } = &s.action else {
            panic!("expected command");
        };
        assert_eq!(glob, &None);
        assert_eq!(types, &vec!["python".to_string()]);
        assert_eq!(check, "uv run pytest -k 'not slow' {{files}}");
        assert!(matches!(s.exclude, Some(Exclude::Global)));
    }

    #[test]
    fn delegated_duplicate_ids_run_once() {
        let m = migrate(
            r#"
repos:
- repo: https://github.com/pre-commit/mirrors-mypy
  rev: v1.0.0
  hooks:
  - id: mypy
  - id: mypy
    additional_dependencies: [types-requests]
"#,
        );
        let steps = &m.stages["pre-commit"];
        assert_eq!(steps.len(), 1);
        assert!(matches!(steps["mypy"].action, Action::Delegate(_)));
    }

    #[test]
    fn legacy_stage_names_are_normalized() {
        let m = migrate(
            r#"
repos:
- repo: local
  hooks:
  - id: a
    entry: a
    language: system
    stages: [commit, push, manual]
"#,
        );
        assert!(m.stages.contains_key("pre-commit"));
        assert!(m.stages.contains_key("pre-push"));
        assert!(m.stages.contains_key("manual"));
        assert!(!m.stages.contains_key("commit-msg"));
    }

    #[test]
    fn default_stages_skip_message_hooks() {
        let m = migrate(
            r#"
default_install_hook_types: [pre-commit, commit-msg, pre-push]
repos:
- repo: https://github.com/pre-commit/pre-commit-hooks
  rev: v5.0.0
  hooks:
  - id: trailing-whitespace
"#,
        );
        assert!(m.stages["pre-commit"].contains_key("trailing-whitespace"));
        assert!(m.stages["pre-push"].contains_key("trailing-whitespace"));
        assert!(!m.stages.contains_key("commit-msg"));
    }

    #[test]
    fn always_run_with_filenames_is_delegated() {
        let m = migrate(
            r#"
repos:
- repo: local
  hooks:
  - id: a
    entry: a
    language: system
    always_run: true
  - id: b
    entry: b
    language: system
    files: \.py$
    always_run: true
    pass_filenames: false
"#,
        );
        assert!(matches!(
            step(&m, "pre-commit", "a").action,
            Action::Delegate(_)
        ));
        let Action::Command { glob, check, .. } = &step(&m, "pre-commit", "b").action else {
            panic!("expected command");
        };
        assert_eq!(glob, &None);
        assert_eq!(check, "b");
    }

    #[test]
    fn types_translation() {
        let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            translate_types(&s(&["file", "python"]), &[]),
            Ok(s(&["python"]))
        );
        assert_eq!(
            translate_types(&s(&["text", "ts"]), &[]),
            Ok(s(&["typescript"]))
        );
        assert_eq!(
            translate_types(&[], &s(&["yaml", "json"])),
            Ok(s(&["yaml", "json"]))
        );
        assert!(translate_types(&s(&["python", "executable"]), &[]).is_err());
        assert!(translate_types(&s(&["cobol"]), &[]).is_err());
    }

    #[test]
    fn pkl_strings() {
        assert_eq!(pkl_str("plain"), "\"plain\"");
        assert_eq!(pkl_str(r"^docs/.*\.md$"), r##"#"^docs/.*\.md$"#"##);
        assert_eq!(pkl_str("a\"#b\\"), "##\"a\"#b\\\"##");
        assert_eq!(pkl_str("(?x)\n^a\n"), "\"\"\"\n(?x)\n^a\n\"\"\"");
    }
}
