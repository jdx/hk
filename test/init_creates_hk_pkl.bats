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
    assert_file_contains hk.pkl "linters ="
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
    assert_output --partial "Detected: hadolint (Dockerfile)"
    assert_file_contains hk.pkl "Builtins.hadolint"
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
    assert_file_contains hk.pkl "hooks"
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
