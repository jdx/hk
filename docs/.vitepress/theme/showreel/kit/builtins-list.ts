// Today's builtin identifiers, frozen: the texture the config scene's rivers
// stream (storyboard §6.2). They are names to glimpse, not to count, so the
// reel shows no total and a builtin added later never has to be here; a
// test only checks that every name still exists.
//
// Each name is how an hk.pkl refers to it, `Builtins.<name>`:
// scripts/gen_builtins.py makes the identifier from the file stem in
// pkl/builtins/ with every `-` turned into `_`, so
// pkl/builtins/editorconfig-checker.pkl is `Builtins.editorconfig_checker`.
// Taken from `hk builtins` (facts.md §4), in code-point order.

export const BUILTINS: readonly string[] = Object.freeze([
  "actionlint", "alejandra", "aqua_update_checksum", "asciidoctor", "astro", "betterleaks", "biome", "black",
  "brakeman", "buf_format", "buf_lint", "buildifier_format", "buildifier_lint", "bundle_audit",
  "byte_order_marker", "cargo_check", "cargo_clippy", "cargo_deny", "cargo_fmt", "check_added_large_files",
  "check_case_conflict", "check_conventional_commit", "check_executables_have_shebangs",
  "check_merge_conflict", "check_shebang_scripts_are_executable", "check_symlinks", "clang_format",
  "cmake_format", "cocogitto_commit_msg", "contextlint", "cpp_lint", "dclint", "deadnix", "deno", "deno_check",
  "destroyed_symlinks", "detect_private_key", "dotnet_format", "dprint", "droast", "editorconfig_checker",
  "erb", "err_check", "eslint", "fasterer", "fix_smart_quotes", "flake8", "forbid_submodules",
  "ghalint_action", "ghalint_workflow", "gitleaks", "go_fix", "go_fmt", "go_fumpt", "go_imports", "go_lines",
  "go_sec", "go_vet", "go_vuln_check", "golangci_lint", "golangci_lint_fmt", "gomod_tidy",
  "google_java_format", "hadolint", "harper", "harper_commit_message", "hclfmt", "hk_test", "isort", "jq",
  "just_format", "kingfisher", "knip", "ktlint", "kube_linter", "kubeconform", "ls_lint", "luacheck", "lychee",
  "lychee_extended", "mado", "markdown_lint", "mdschema", "mise", "mix_compile", "mix_fmt", "mix_test",
  "mixed_line_ending", "mypy", "newlines", "nil", "nix_fmt", "nixf_diagnose", "nixpkgs_format",
  "no_commit_to_branch", "ox_lint", "oxfmt", "php_cs", "pinact", "pinact_update", "pkl", "pkl_format",
  "prettier", "pylint", "python_check_ast", "python_debug_statements", "reek", "renovate_deps", "revive",
  "rubocop", "rubocop_server", "ruff", "ruff_format", "rumdl", "rumdl_format", "rustfmt", "ryl",
  "ryl_markdown", "selene", "shellcheck", "shellharden", "sherif", "shfmt", "sorbet", "sort_package_json",
  "sql_fluff", "standard_js", "standard_rb", "staticcheck", "stylelint", "stylua", "swiftlint", "taplo",
  "taplo_format", "terraform", "terraform_docs", "terraform_validate", "terragrunt_hcl_fmt",
  "terragrunt_hcl_validate", "textlint", "tf_lint", "tofu", "tombi", "tombi_format", "trailing_whitespace",
  "tsc", "tsserver", "ty", "typos", "vacuum", "vale", "vp_check", "vp_fmt", "vp_lint", "xmllint", "xo",
  "yamlfmt", "yamllint", "yq", "zizmor",
]);

/** The two that pop out of the rivers under "Builtins, from `prettier` to `zizmor`." */
export const BUILTIN_POPS = ["prettier", "zizmor"] as const;
