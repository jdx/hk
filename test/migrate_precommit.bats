#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "migrate from pre-commit - basic config" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
    -   id: eslint
PRECOMMIT

    run hk migrate from-precommit
    assert_success
    assert_output --partial "Successfully migrated to hk.pkl"
    
    # Verify hk.pkl was created
    [ -f hk.pkl ]
    
    # Verify it contains expected content
    run cat hk.pkl
    assert_output --partial "Builtins.prettier"
    assert_output --partial "Builtins.eslint"
    assert_output --partial 'hooks {'
    assert_output --partial '["pre-commit"]'
}

@test "migrate from pre-commit - with exclude" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/asottile/reorder-python-imports
    rev: v3.15.0
    hooks:
    -   id: reorder-python-imports
        exclude: ^(pre_commit/resources/)
PRECOMMIT

    run hk migrate from-precommit
    assert_success
    
    # Verify exclude is preserved
    run cat hk.pkl
    assert_output --partial 'exclude = "^(pre_commit/resources/)"'
}

@test "migrate from pre-commit - with args" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/asottile/pyupgrade
    rev: v3.20.0
    hooks:
    -   id: pyupgrade
        args: [--py39-plus]
PRECOMMIT

    run hk migrate from-precommit
    assert_success
    
    # Verify args are noted in comments
    run cat hk.pkl
    assert_output --partial "args from pre-commit: --py39-plus"
}

@test "migrate from pre-commit - unknown hook" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/some/unknown-hook
    rev: v1.0.0
    hooks:
    -   id: unknown-linter
PRECOMMIT

    run hk migrate from-precommit
    assert_success
    
    # Verify unknown hooks are in custom_steps with TODO
    run cat hk.pkl
    assert_output --partial "custom_steps"
    assert_output --partial "TODO: Configure check and/or fix commands"
    assert_output --partial "Repo: https://github.com/some/unknown-hook @ v1.0.0"
}

@test "migrate from pre-commit - force overwrite" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: black
PRECOMMIT

    # Create existing hk.pkl
    echo "existing content" > hk.pkl
    
    # Try without force - should fail
    run hk migrate from-precommit
    assert_failure
    assert_output --partial "already exists"
    
    # Try with force - should succeed
    run hk migrate from-precommit --force
    assert_success
    
    run cat hk.pkl
    assert_output --partial "Builtins.black"
}

@test "migrate from pre-commit - custom config path" {
    cat <<PRECOMMIT > custom-precommit.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: shellcheck
PRECOMMIT

    run hk migrate from-precommit --config custom-precommit.yaml --output custom-hk.pkl
    assert_success
    
    # Verify custom output was created
    [ -f custom-hk.pkl ]
    
    run cat custom-hk.pkl
    assert_output --partial "Builtins.shellcheck"
}

@test "migrate from pre-commit - missing config file" {
    run hk migrate from-precommit --config nonexistent.yaml
    assert_failure
    assert_output --partial "does not exist"
}

@test "migrate from pre-commit - mixed known and unknown hooks" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/psf/black
    rev: 23.0.0
    hooks:
    -   id: black
-   repo: https://github.com/PyCQA/flake8
    rev: 7.0.0
    hooks:
    -   id: flake8
-   repo: https://github.com/custom/my-linter
    rev: v1.0.0
    hooks:
    -   id: my-custom-linter
PRECOMMIT

    run hk migrate from-precommit
    assert_success
    
    run cat hk.pkl
    # Verify known hooks
    assert_output --partial "Builtins.black"
    assert_output --partial "Builtins.flake8"
    # Verify unknown hooks
    assert_output --partial "custom_steps"
    assert_output --partial "my-custom-linter"
    # Verify both are used in hooks
    assert_output --partial "...linters"
    assert_output --partial "...custom_steps"
}

@test "migrate from pre-commit - fail_fast config" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
fail_fast: true
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: black
PRECOMMIT

    run hk migrate from-precommit
    assert_success
    
    run cat hk.pkl
    assert_output --partial "fail_fast"
    assert_output --partial "hk uses --fail-fast flag"
}
