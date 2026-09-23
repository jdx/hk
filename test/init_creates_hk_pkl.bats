#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk init creates hk.pkl" {
    hk init
    assert_file_contains hk.pkl "steps {"
}

@test "hk init detects package.json" {
    echo '{"name": "test"}' > package.json
    run hk init
    assert_success
    assert_output --partial "Detected: prettier (package.json)"
    assert_file_contains hk.pkl "Builtins.prettier"
}

@test "hk init detects Cargo.toml" {
    echo '[package]' > Cargo.toml
    echo 'name = "test"' >> Cargo.toml
    run hk init
    assert_success
    assert_output --partial "Detected: cargo_clippy (Cargo.toml)"
    assert_file_contains hk.pkl "Builtins.cargo_clippy"
    assert_file_contains hk.pkl "Builtins.cargo_fmt"
}

@test "hk init detects pyproject.toml" {
    echo '[project]' > pyproject.toml
    echo 'name = "test"' >> pyproject.toml
    run hk init
    assert_success
    assert_output --partial "Detected: ruff (pyproject.toml)"
    assert_file_contains hk.pkl "Builtins.ruff"
}

@test "hk init detects go.mod" {
    echo 'module test' > go.mod
    run hk init
    assert_success
    assert_output --partial "go_fmt (go.mod)"
    assert_output --partial "golangci_lint (go.mod)"
    assert_file_contains hk.pkl "Builtins.golangci_lint"
    assert_file_contains hk.pkl "Builtins.go_fmt"
}

@test "hk init detects GitHub workflows" {
    mkdir -p .github/workflows
    touch .github/workflows/ci.yml
    run hk init
    assert_success
    assert_output --partial "Detected: actionlint (.github/workflows)"
    assert_file_contains hk.pkl "Builtins.actionlint"
    assert_file_contains hk.pkl "Builtins.zizmor"
}

@test "hk init detects Dockerfile" {
    echo 'FROM alpine' > Dockerfile
    run hk init
    assert_success
    assert_output --partial "hadolint (Dockerfile)"
    assert_output --partial "droast (Dockerfile)"
    assert_file_contains hk.pkl "Builtins.hadolint"
    assert_file_contains hk.pkl "Builtins.droast"
}

@test "hk init generates default template when nothing detected" {
    run hk init
    assert_success
    # Should have commented examples
    assert_file_contains hk.pkl "// Add linters here"
}

@test "hk init --force overwrites existing file" {
    echo "old content" > hk.pkl
    run hk init --force
    assert_success
    # Check that old content is gone and new content is present
    run grep "old content" hk.pkl
    assert_failure
    assert_file_contains hk.pkl "steps {"
}

@test "hk init warns if hk.pkl exists without --force" {
    echo "existing content" > hk.pkl
    run hk init
    assert_success
    assert_output --partial "already exists"
    # Should not overwrite
    assert_file_contains hk.pkl "existing content"
}

@test "hk init --mise creates mise.toml" {
    run hk init --mise
    assert_success
    assert_file_exists mise.toml
    assert_file_contains mise.toml "hk = \"latest\""
    run grep -E '(^|:)pkl[" ]*=' mise.toml
    assert_failure
}

@test "hk init detects multiple project types" {
    echo '{"name": "test"}' > package.json
    echo '[package]' > Cargo.toml
    echo 'name = "test"' >> Cargo.toml
    run hk init
    assert_success
    # Should detect both
    assert_output --partial "prettier"
    assert_output --partial "cargo_clippy"
    assert_file_contains hk.pkl "Builtins.prettier"
    assert_file_contains hk.pkl "Builtins.cargo_clippy"
}

@test "hk init mise fills missing entries and preserves existing task" {
    cat > mise.toml <<'TOML'
# keep this comment
[tools]
hk = "3.0"
[ tasks.pre-commit ]
run = "custom-hook"
TOML
    run hk init --mise
    assert_success
    assert_file_contains mise.toml "# keep this comment"
    assert_file_contains mise.toml 'hk = "3.0"'
    assert_file_contains mise.toml 'run = "custom-hook"'
}

@test "hk init mise rejects malformed config without creating hk.pkl" {
    echo '[tools' > mise.toml
    run hk init --mise
    assert_failure
    run test -e hk.pkl
    assert_failure
}

@test "hk init mise force does not reset mise config" {
    echo 'custom = true' > mise.toml
    echo 'old' > hk.pkl
    run hk init --mise --force
    assert_success
    assert_file_contains mise.toml 'custom = true'
    assert_file_contains mise.toml 'hk = "latest"'
    run grep old hk.pkl
    assert_failure
}

@test "hk init mise preserves string and inline task forms" {
    cat > mise.toml <<'TOML'
[tools]
hk = "3.1"
prettier = "latest"
[tasks]
pre-commit = "custom-command"
check = { run = "custom-check" }
TOML
    run hk init --mise
    assert_success
    assert_file_contains mise.toml 'pre-commit = "custom-command"'
    assert_file_contains mise.toml 'check = { run = "custom-check" }'
}

@test "hk init mise is byte-identical on repeated force" {
    run hk init --mise --force
    assert_success
    cp mise.toml mise.before
    run hk init --mise --force
    assert_success
    cmp mise.before mise.toml
}

@test "hk init mise preserves qualified hk tool key" {
    cat > mise.toml <<'TOML'
[tools]
"aqua:jdx/hk" = "1.0"
TOML
    run hk init --mise
    assert_success
    assert_file_contains mise.toml '"aqua:jdx/hk" = "1.0"'
    run grep '^hk = ' mise.toml
    assert_failure
}

@test "hk init mise rejects unsupported tools without changing files" {
    echo 'tools = "custom"' > mise.toml
    echo old > hk.pkl
    cp mise.toml mise.before
    run hk init --mise --force
    assert_failure
    cmp mise.before mise.toml
    assert_file_contains hk.pkl old
}

@test "hk init mise merges inline tools and tasks" {
    cat > mise.toml <<'TOML'
tools = { hk = "3.1" }
tasks = { check = "custom" }
TOML
    run hk init --mise --force
    assert_success
    assert_file_contains mise.toml 'hk = "3.1"'
    assert_file_contains mise.toml 'check = "custom"'
    assert_file_contains mise.toml 'pre-commit'
    cp mise.toml mise.before
    run hk init --mise --force
    assert_success
    cmp mise.before mise.toml
}

@test "hk init mise rejects scalar tasks without changing files" {
    cat > mise.toml <<'TOML'
tools = { hk = "3.1" }
tasks = "custom"
TOML
    echo sentinel > hk.pkl
    cp mise.toml mise.before
    run hk init --mise --force
    assert_failure
    cmp mise.before mise.toml
    assert_file_contains hk.pkl sentinel
}

@test "hk init mise preserves dependencies-only pre-commit task" {
    cat > mise.toml <<'TOML'
[tasks.pre-commit]
depends = ["check"]
TOML
    run hk init --mise
    assert_success
    run grep '^run = ' mise.toml
    assert_failure
    run grep -F 'depends = ["check"]' mise.toml
    assert_success
}

@test "hk init mise malformed config preserves existing hk.pkl" {
    echo '[tools' > mise.toml
    echo sentinel > hk.pkl
    cp mise.toml mise.before
    run hk init --mise --force
    assert_failure
    cmp mise.before mise.toml
    assert_file_contains hk.pkl sentinel
}

@test "hk init mise preserves existing file task" {
    cat > mise.toml <<'TOML'
[tools]
hk = "3.1"
pkl = "0.26"
TOML
    cp mise.toml mise.before
    mkdir -p mise-tasks
    echo '#!/bin/sh' > mise-tasks/pre-commit
    chmod +x mise-tasks/pre-commit
    run hk init --mise
    assert_success
    cmp mise.before mise.toml
}

@test "hk init mise inserts pre-commit with included task configuration" {
    cat > mise.toml <<'TOML'
[tools]
hk = "latest"
[task_config]
includes = ["custom-tasks"]
TOML
    cat > custom-tasks <<'TOML'
[build]
run = "echo build"
TOML
    run hk init --mise
    assert_success
    run grep -F 'includes = ["custom-tasks"]' mise.toml
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_success
    run grep -F 'run = "hk run pre-commit"' mise.toml
    assert_success
}

@test "hk init mise still inserts pre-commit without external task configuration" {
    cat > mise.toml <<'TOML'
[tools]
hk = "3.1"
TOML
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_success
    assert_file_contains mise.toml 'run = "hk run pre-commit"'
}

@test "hk init mise recognizes qualified pkl tool" {
    cat > mise.toml <<'TOML'
[tools]
"github:apple/pkl" = "0.26"
TOML
    run hk init --mise
    assert_success
    run grep '^pkl = ' mise.toml
    assert_failure
    assert_file_contains mise.toml '"github:apple/pkl" = "0.26"'
}

@test "hk init mise preserves included pre-commit task" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks.toml"]
TOML
    cat > tasks.toml <<'TOML'
[pre-commit]
run = "echo included"
TOML
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise inserts with unrelated included task file" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks.toml"]
TOML
    cat > tasks.toml <<'TOML'
[build]
run = "echo build"
TOML
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_success
}

@test "hk init mise preserves included task directory" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks"]
TOML
    mkdir -p tasks/pre-commit
    touch tasks/pre-commit/_default
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise inserts with unrelated included toml" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks"]
TOML
    mkdir -p tasks
    cat > tasks/pre-commit.toml <<'TOML'
[build]
run = "echo build"
TOML
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_success
}

@test "hk init mise preserves pre-commit in included task map" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks"]
TOML
    mkdir -p tasks
    cat > tasks/hooks.toml <<'TOML'
[pre-commit]
run = "echo included"
TOML
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise preserves nested included task map" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks"]
TOML
    mkdir -p tasks/nested
    cat > tasks/nested/hooks.toml <<'TOML'
[pre-commit]
run = "echo included"
TOML
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise warns for malformed included task map" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks"]
TOML
    mkdir -p tasks
    printf '[broken\n' > tasks/hooks.toml
    run hk init --mise
    assert_success
    assert_output --partial "Unable to inspect mise task includes"
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise warns for symlinked included task" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks"]
TOML
    mkdir -p real-tasks tasks
    ln -s "../real-tasks" tasks/link
    run hk init --mise
    assert_success
    assert_output --partial "Unable to inspect mise task includes"
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise suppresses insertion for unreadable include" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["missing.toml"]
TOML
    run hk init --mise
    assert_success
    assert_output --partial "Unable to inspect mise task includes"
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise rejects scalar task_config before force writes" {
    cat > mise.toml <<'TOML'
task_config = "invalid"
TOML
    echo sentinel > hk.pkl
    cp mise.toml mise.before
    run hk init --mise --force
    assert_failure
    assert_output --partial "unsupported mise.toml: [task_config] must be a table"
    cmp mise.before mise.toml
    assert_file_contains hk.pkl sentinel
}

@test "hk init mise suppresses insertion for dynamic include" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["{root}/tasks.toml"]
TOML
    run hk init --mise
    assert_success
    assert_output --partial "Unable to inspect mise task includes"
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise suppresses insertion for malformed include" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks.toml"]
TOML
    printf '[broken\n' > tasks.toml
    run hk init --mise
    assert_success
    assert_output --partial "Unable to inspect mise task includes"
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_failure
}

@test "hk init mise includes replace conventional task defaults" {
    cat > mise.toml <<'TOML'
[task_config]
includes = ["tasks.toml"]
TOML
    cat > tasks.toml <<'TOML'
[build]
run = "echo build"
TOML
    mkdir -p mise-tasks
    touch mise-tasks/pre-commit
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_success
}

@test "hk init mise allows empty includes" {
    cat > mise.toml <<'TOML'
[task_config]
includes = []
TOML
    run hk init --mise
    assert_success
    run grep -F '[tasks.pre-commit]' mise.toml
    assert_success
}
