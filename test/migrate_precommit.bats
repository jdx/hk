#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

migrate() {
    run hk migrate pre-commit --hk-pkl-root "$PKL_PATH" "$@"
}

@test "migrate precommit - known hooks become builtins" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
fail_fast: true
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
    -   id: trailing-whitespace
    -   id: end-of-file-fixer
-   repo: https://github.com/astral-sh/ruff-pre-commit.git
    rev: v0.6.9
    hooks:
    -   id: ruff
        args: [--fix]
    -   id: ruff-format
-   repo: meta
    hooks:
    -   id: check-useless-excludes
PRECOMMIT

    migrate
    assert_success
    assert_output --partial "4 builtins, 0 commands, 0 run through"

    run cat hk.pkl
    assert_output --partial '["trailing-whitespace"] = Builtins.trailing_whitespace'
    assert_output --partial '["end-of-file-fixer"] = Builtins.newlines'
    assert_output --partial '["ruff"] = Builtins.ruff'
    assert_output --partial '["ruff-format"] = Builtins.ruff_format'
    refute_output --partial 'check-useless-excludes'
    refute_output --partial 'precommit('
    refute_output --partial 'fail_fast'
    refute_output --partial 'hooks {'

    run hk validate
    assert_success
}

@test "migrate precommit - hooks run through the runner when hk has no equivalent" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
repos:
-   repo: https://github.com/codespell-project/codespell
    rev: v2.3.0
    hooks:
    -   id: codespell
-   repo: https://github.com/psf/black
    rev: 24.8.0
    hooks:
    -   id: black
        args: [--line-length, 100]
-   repo: https://github.com/example/hooks
    rev: v1.0.0
    hooks:
    -   id: black
-   repo: local
    hooks:
    -   id: lint-py
        name: lint
        entry: python scripts/lint.py
        language: python
        additional_dependencies: [requests]
PRECOMMIT

    migrate --runner pre-commit
    assert_success
    assert_output --partial "0 builtins, 0 commands, 3 run through pre-commit"
    assert_output --partial "Keep .pre-commit-config.yaml and pre-commit installed for: codespell, black, lint-py"

    run cat hk.pkl
    assert_output --partial 'check = "pre-commit run --hook-stage \(stage) \(hook)"'
    assert_output --partial '// no hk builtin for codespell from https://github.com/codespell-project/codespell'
    assert_output --partial '["codespell"] = precommit("codespell", "pre-commit")'
    assert_output --partial "// Builtins.black does not support this hook's args (--line-length 100)"
    assert_output --partial '// pre-commit sets up a python environment for this hook'
    assert_output --partial '["lint-py"] = precommit("lint-py", "pre-commit")'
    # `pre-commit run black` runs both black entries, so there is one step
    run grep -c '"black"' hk.pkl
    assert_output "1"

    # The step calls the runner with the hook id and hk's files
    mkdir -p bin
    cat <<'SH' > bin/pre-commit
#!/usr/bin/env bash
echo "$*" >> "$(dirname "$0")/../calls.txt"
SH
    chmod +x bin/pre-commit
    echo "hello" > README.md
    printf '\x89PNG\r\n\x1a\n\x00\x00' > logo.png
    git add README.md logo.png
    PATH="$PWD/bin:$PATH" run hk check --all --step codespell
    assert_success
    run cat calls.txt
    assert_output --partial "run --hook-stage pre-commit codespell --files"
    assert_output --partial "README.md"
    # The runner applies the hook's own filters, so binaries reach it too
    assert_output --partial "logo.png"
}

@test "migrate precommit - custom config path is passed to the runner" {
    mkdir -p config
    cat <<'PRECOMMIT' > "config/team's pre-commit.yaml"
repos:
-   repo: https://github.com/codespell-project/codespell
    rev: v2.3.0
    hooks:
    -   id: codespell
PRECOMMIT

    migrate --config "config/team's pre-commit.yaml" --runner prek
    assert_success
    run cat hk.pkl
    assert_output --partial "check = \"prek run --config 'config/team'\\\\''s pre-commit.yaml' --hook-stage"
    run hk validate
    assert_success
}

@test "migrate precommit - local system hooks become commands" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
repos:
-   repo: local
    hooks:
    -   id: pytest
        name: pytest
        entry: uv run pytest
        language: system
        types: [file, python]
        args: [-k, "not slow"]
    -   id: no-rej
        name: no .rej files
        entry: Remove .rej files
        language: fail
        files: \.rej$
    -   id: generate
        name: generate
        entry: ./scripts/generate.sh
        language: script
        always_run: true
        pass_filenames: false
PRECOMMIT

    migrate
    assert_success
    assert_output --partial "0 builtins, 3 commands, 0 run through"

    run cat hk.pkl
    assert_output --partial "types = List(\"python\")"
    assert_output --partial "check = \"uv run pytest -k 'not slow' {{files}}\""
    assert_output --partial 'glob = Regex(#"\.rej$"#)'
    assert_output --partial 'check = "./scripts/generate.sh"'

    run hk validate
    assert_success

    touch patch.rej
    git add patch.rej
    run hk check --all --step no-rej
    assert_failure
    assert_output --partial "Remove .rej files"
    assert_output --partial "patch.rej"
}

@test "migrate precommit - filters hk cannot express are delegated" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
repos:
-   repo: local
    hooks:
    -   id: lookahead
        name: lookahead
        entry: echo
        language: system
        files: ^(?!vendor/).*\.py$
    -   id: exclude-types
        name: exclude types
        entry: echo
        language: system
        exclude_types: [markdown]
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
    -   id: trailing-whitespace
        files: ^src/
PRECOMMIT

    migrate
    assert_success
    run cat hk.pkl
    assert_output --partial "// its \`files\` regex uses syntax hk's regex engine does not support"
    assert_output --partial '// `exclude_types` has no hk equivalent'
    assert_output --partial "// Builtins.trailing_whitespace does not support this hook's \`files\`"
    run hk validate
    assert_success
}

@test "migrate precommit - top-level and hook excludes are combined" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
exclude: |
    (?x)^(
        vendor/.*|  # third party code
        dist/.*
    )$
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
    -   id: trailing-whitespace
    -   id: end-of-file-fixer
        exclude: \.snap$
PRECOMMIT

    migrate
    assert_success
    run cat hk.pkl
    assert_output --partial 'local excluded = Regex("""'
    assert_output --partial 'exclude = excluded'
    assert_output --partial '(?:\.snap$)|(?:(?x)^('

    run hk validate
    assert_success

    mkdir -p vendor src
    printf 'trailing   \n' > vendor/lib.txt
    printf 'clean\n' > src/app.txt
    printf 'snapshot' > src/app.snap
    git add vendor src
    run hk check --all
    assert_success
}

@test "migrate precommit - stages map to hk hooks" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
default_install_hook_types: [pre-commit, commit-msg, pre-merge-commit]
default_stages: [pre-commit, pre-push, pre-merge-commit]
repos:
-   repo: https://github.com/compilerla/conventional-pre-commit
    rev: v3.4.0
    hooks:
    -   id: conventional-pre-commit
-   repo: local
    hooks:
    -   id: no-wip
        name: no wip
        entry: sh -c '! grep -qi wip "$1"' --
        language: system
        stages: [commit-msg]
    -   id: default
        name: default
        entry: echo default
        language: system
    -   id: legacy
        name: legacy
        entry: echo legacy
        language: system
        stages: [commit, push]
    -   id: docs
        name: docs
        entry: echo docs
        language: system
        stages: [manual]
PRECOMMIT

    migrate
    assert_success
    assert_output --partial "hk has no pre-merge-commit hook, so these hooks will not run at that stage: default"

    run cat hk.pkl
    assert_output --partial '["conventional-pre-commit"] = Builtins.check_conventional_commit'
    assert_output --partial "check = #\"sh -c '! grep -qi wip \"\$1\"' -- {{commit_msg_file}}\"#"
    assert_output --partial '["pre-push"]'
    assert_output --partial 'local manual_steps = new Mapping<String, Step> {'
    assert_output --partial '["check"] { steps { ...manual_steps } }'
    assert_output --partial '["fix"] { steps { ...manual_steps } }'
    # default_stages only apply to hook types pre-commit installs
    run grep -c '"default"' hk.pkl
    assert_output "1"

    run hk validate
    assert_success

    echo "feat: add thing" > msg
    run hk run commit-msg msg
    assert_success
    echo "feat: WIP thing" > msg
    run hk run commit-msg msg
    assert_failure
    echo "not conventional" > msg
    run hk run commit-msg msg
    assert_failure
}

@test "migrate precommit - keeps pre-commit's run-every-hook behavior" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
    -   id: trailing-whitespace
PRECOMMIT

    migrate
    assert_success
    run cat hk.pkl
    assert_output --partial 'fail_fast = false'
}

@test "migrate precommit - numeric args are accepted" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
repos:
-   repo: https://github.com/PyCQA/flake8
    rev: 7.0.0
    hooks:
    -   id: flake8
        args: [--max-line-length, 100]
PRECOMMIT

    migrate
    assert_success
    run cat hk.pkl
    assert_output --partial "does not support this hook's args (--max-line-length 100)"
}

@test "migrate precommit - refuses to overwrite without --force" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
    -   id: trailing-whitespace
PRECOMMIT
    echo "existing" > hk.pkl

    migrate
    assert_failure
    assert_output --partial "already exists"

    migrate --force
    assert_success
    run cat hk.pkl
    assert_output --partial "Builtins.trailing_whitespace"
}

@test "migrate precommit - custom output path" {
    cat <<'PRECOMMIT' > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v5.0.0
    hooks:
    -   id: trailing-whitespace
PRECOMMIT

    migrate --output hk.migrated.pkl
    assert_success
    [ -f hk.migrated.pkl ]
    [ ! -f hk.pkl ]
}

@test "migrate precommit - missing config file" {
    migrate
    assert_failure
    assert_output --partial ".pre-commit-config.yaml does not exist"
}

@test "migrate precommit - apache airflow real-world config" {
    command -v curl &> /dev/null || skip "curl not available"
    curl -sfo .pre-commit-config.yaml https://raw.githubusercontent.com/apache/airflow/main/.pre-commit-config.yaml ||
        skip "Failed to download Airflow config"

    migrate
    assert_success
    run hk validate
    assert_success
}
