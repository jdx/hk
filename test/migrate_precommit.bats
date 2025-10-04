#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "migrate precommit - basic config" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
    -   id: eslint
PRECOMMIT

    run hk migrate pre-commit
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

@test "migrate precommit - with exclude" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/asottile/reorder-python-imports
    rev: v3.15.0
    hooks:
    -   id: reorder-python-imports
        exclude: ^(pre_commit/resources/)
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify exclude is preserved (as regex)
    run cat hk.pkl
    assert_output --partial 'exclude = Types.Regex'
    assert_output --partial 'pre_commit/resources'
}

@test "migrate precommit - with args" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/asottile/pyupgrade
    rev: v3.20.0
    hooks:
    -   id: pyupgrade
        args: [--py39-plus]
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify args are noted in comments
    # The hook is unknown so it goes to custom_steps
    run cat hk.pkl
    assert_output --partial "custom_steps"
    assert_output --partial "pyupgrade"
}

@test "migrate precommit - with additional_dependencies and mise x" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/mirrors-mypy
    rev: v1.0.0
    hooks:
    -   id: mypy
        additional_dependencies: [types-pyyaml, types-requests]
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify additional_dependencies are handled with mise x
    run cat hk.pkl
    assert_output --partial "additional_dependencies: types-pyyaml, types-requests"
    assert_output --partial 'prefix = "mise x mypy@latest --"'
}

@test "migrate precommit - with types and type filtering" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
        types: [javascript, typescript]
        exclude_types: [markdown]
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify type filtering is documented
    run cat hk.pkl
    assert_output --partial "types (AND): javascript, typescript"
    assert_output --partial "exclude_types: markdown"
}

@test "migrate precommit - with stages" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
        stages: [pre-push]
    -   id: eslint
        stages: [pre-commit]
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify both stages are created
    run cat hk.pkl
    assert_output --partial '["pre-push"]'
    assert_output --partial '["pre-commit"]'
}

@test "migrate precommit - local repo" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: local
    hooks:
    -   id: my-local-check
        name: My Local Check
        entry: ./scripts/check.sh
        language: system
        files: \.py$
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify local hooks are generated with check command
    run cat hk.pkl
    assert_output --partial "local_hooks"
    assert_output --partial "my-local-check"
    assert_output --partial 'check = "./scripts/check.sh {{files}}"'
}

@test "migrate precommit - local hook with pass_filenames false" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: local
    hooks:
    -   id: test
        name: Run tests
        entry: cargo test
        language: system
        files: '\.rs$'
        pass_filenames: false
PRECOMMIT

    run hk migrate pre-commit
    assert_success

    # Verify local hook without {{files}}
    run cat hk.pkl
    assert_output --partial "local_hooks"
    assert_output --partial 'check = "cargo test"'
    refute_output --partial "{{files}}"
    assert_output --partial "pass_filenames was false"
}

@test "migrate precommit - meta repo is skipped" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: meta
    hooks:
    -   id: check-hooks-apply
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify meta hooks are not included
    run cat hk.pkl
    refute_output --partial "check-hooks-apply"
    assert_output --partial "Builtins.prettier"
}

@test "migrate precommit - unknown hook" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/some/unknown-hook
    rev: v1.0.0
    hooks:
    -   id: unknown-linter
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    # Verify unknown hooks are in custom_steps with TODO
    run cat hk.pkl
    assert_output --partial "custom_steps"
    assert_output --partial "TODO: Configure check and/or fix commands"
    assert_output --partial "Repo: https://github.com/some/unknown-hook @ v1.0.0"
}

@test "migrate precommit - force overwrite" {
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
    run hk migrate pre-commit
    assert_failure
    assert_output --partial "already exists"
    
    # Try with force - should succeed
    run hk migrate pre-commit --force
    assert_success
    
    run cat hk.pkl
    assert_output --partial "Builtins.black"
}

@test "migrate precommit - custom config path" {
    cat <<PRECOMMIT > custom-precommit.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: shellcheck
PRECOMMIT

    run hk migrate pre-commit --config custom-precommit.yaml --output custom-hk.pkl
    assert_success
    
    # Verify custom output was created
    [ -f custom-hk.pkl ]
    
    run cat custom-hk.pkl
    assert_output --partial "Builtins.shellcheck"
}

@test "migrate precommit - missing config file" {
    run hk migrate pre-commit --config nonexistent.yaml
    assert_failure
    assert_output --partial "does not exist"
}

@test "migrate precommit - mixed known and unknown hooks" {
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

    run hk migrate pre-commit
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

@test "migrate precommit - fail_fast config" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
fail_fast: true
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: black
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    run cat hk.pkl
    assert_output --partial "fail_fast"
    assert_output --partial "hk uses --fail-fast"
}

@test "migrate precommit - default_language_version" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
default_language_version:
    python: python3.11
    node: 18.0.0
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: black
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    run cat hk.pkl
    assert_output --partial "default_language_version"
    assert_output --partial "python: python3.11"
    assert_output --partial "node: 18.0.0"
    assert_output --partial "mise use python@3.11"
    assert_output --partial "mise use node@18.0.0"
}

@test "migrate precommit - always_run flag" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
        always_run: true
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    run cat hk.pkl
    assert_output --partial "always_run: true"
}

@test "migrate precommit - pass_filenames false" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: black
        pass_filenames: false
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    
    run cat hk.pkl
    assert_output --partial "pass_filenames: false"
    assert_output --partial "not use {{files}}"
}

@test "migrate precommit - generates check and fix hooks" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
PRECOMMIT

    run hk migrate pre-commit
    assert_success

    run cat hk.pkl
    # Should have pre-commit, check, and fix hooks
    assert_output --partial '["pre-commit"]'
    assert_output --partial '["check"]'
    assert_output --partial '["fix"]'
}

@test "migrate precommit - apache airflow real-world config" {
    # Test with actual Apache Airflow pre-commit config
    if ! command -v curl &> /dev/null; then
        skip "curl not available"
    fi

    curl -s -o .pre-commit-config.yaml https://raw.githubusercontent.com/apache/airflow/main/.pre-commit-config.yaml || skip "Failed to download Airflow config"

    # Verify we downloaded something
    [ -f .pre-commit-config.yaml ]
    [ -s .pre-commit-config.yaml ]

    run hk migrate pre-commit
    assert_success
    assert_output --partial "Successfully migrated to hk.pkl"

    # Verify hk.pkl was created
    [ -f hk.pkl ]

    # Verify basic structure
    run cat hk.pkl
    assert_output --partial 'import "package://github.com/jdx/hk'
    assert_output --partial 'hooks {'

    # Apache Airflow uses several common pre-commit hooks
    # These may change over time, so we just check for some basic patterns
    # rather than specific hooks
    assert_output --regexp 'Builtins\.(yamllint|check_merge_conflict|mixed_line_ending|trailing_whitespace|detect_private_key|newlines|python_debug_statements|check_executables_have_shebangs)'

    # Verify it has both pre-commit and pre-push stages (Airflow uses default_stages)
    assert_output --partial '["pre-commit"]'
    assert_output --partial '["pre-push"]'

    # Verify it has local hooks section
    assert_output --partial 'local local_hooks'

    # Verify it has custom steps for unmapped hooks
    assert_output --partial 'local custom_steps'
}

@test "migrate precommit - vendor external repo hooks" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/Lucas-C/pre-commit-hooks
    rev: v1.5.5
    hooks:
    -   id: remove-crlf
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    assert_output --partial "Successfully migrated to hk.pkl"
    
    # Verify .hk directory was created with vendored repo
    [ -d .hk/vendors ]
    [ -d .hk/vendors/Lucas-C-pre-commit-hooks ]
    [ -f .hk/vendors/Lucas-C-pre-commit-hooks/.pre-commit-hooks.yaml ]
    
    # Verify .git directory was removed
    [ ! -d .hk/vendors/Lucas-C-pre-commit-hooks/.git ]
    
    # Verify hk.pkl references vendored hooks
    run cat hk.pkl
    assert_output --partial 'import ".hk/vendors/Lucas-C-pre-commit-hooks/hooks.pkl"'
    assert_output --partial "remove-crlf"
    assert_output --partial "Builtins.prettier"
    
    # Verify vendored PKL file was created
    [ -f .hk/vendors/Lucas-C-pre-commit-hooks/hooks.pkl ]
    
    # Verify the generated PKL file has correct structure
    run cat .hk/vendors/Lucas-C-pre-commit-hooks/hooks.pkl
    assert_output --partial "remove_crlf"
    
    # Verify hooks use the vendored scripts and are in linters
    run cat hk.pkl
    assert_output --partial "local linters"
    assert_output --partial "Vendors_Lucas_C_pre_commit_hooks.remove_crlf"
    refute_output --partial "custom_steps"
    
    # Create a test file with CRLF line endings to test the vendored hook
    printf "line1\r\nline2\r\n" > test.txt
    git add test.txt
    
    # Dependencies will be installed automatically when the hook runs
    
    # Run hk fix - should install dependencies and remove CRLF
    run hk fix
    assert_success
    
    # Verify CRLF was removed - check file directly for \r bytes
    run od -c test.txt
    refute_output --partial '\r'
}

@test "migrate precommit - vendor node hooks (doctoc)" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/thlorenz/doctoc
    rev: v2.2.0
    hooks:
    -   id: doctoc
        args: [--maxlevel=3]
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    assert_output --partial "Successfully migrated to hk.pkl"
    
    # Verify .hk directory was created with vendored repo
    [ -d .hk/vendors ]
    [ -d .hk/vendors/thlorenz-doctoc ]
    [ -f .hk/vendors/thlorenz-doctoc/.pre-commit-hooks.yaml ]
    
    # Verify .git directory was removed
    [ ! -d .hk/vendors/thlorenz-doctoc/.git ]
    
    # Verify package.json exists in the vendored repo
    [ -f .hk/vendors/thlorenz-doctoc/package.json ]
    
    # Verify hk.pkl references vendored hooks
    run cat hk.pkl
    assert_output --partial 'import ".hk/vendors/thlorenz-doctoc/hooks.pkl"'
    assert_output --partial "doctoc"
    assert_output --partial "Builtins.prettier"
    
    # Verify vendored PKL file was created
    [ -f .hk/vendors/thlorenz-doctoc/hooks.pkl ]
    
    # Verify the generated PKL file has correct structure for Node.js
    run cat .hk/vendors/thlorenz-doctoc/hooks.pkl
    assert_output --partial "doctoc"
    assert_output --partial "node_modules"
    assert_output --partial "npm install"
    assert_output --partial "npx --prefix .hk/vendors/thlorenz-doctoc doctoc"
    
    # Verify hooks use the vendored scripts and are in linters
    run cat hk.pkl
    assert_output --partial "local linters"
    assert_output --partial "Vendors_thlorenz_doctoc.doctoc"
    refute_output --partial "custom_steps"
    
    # Create a test markdown file without TOC to test the vendored hook
    cat <<'MARKDOWN' > README.md
# Test Document

This is a test document for doctoc.

## Section One

Some content here.

### Subsection 1.1

More content.

## Section Two

Final section.
MARKDOWN
    git add README.md
    
    # Dependencies will be installed automatically when the hook runs
    
    # Run hk fix - should install dependencies and add TOC to README.md
    run hk fix
    assert_success
    
    # Verify TOC was added to the markdown file
    run cat README.md
    assert_output --partial "<!-- START doctoc"
    assert_output --partial "<!-- END doctoc"
    assert_output --partial "Section One"
    assert_output --partial "Section Two"
}

@test "migrate precommit - vendor golang hooks (buf)" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/bufbuild/buf
    rev: v1.47.2
    hooks:
    -   id: buf-format
    -   id: buf-lint
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    assert_output --partial "Successfully migrated to hk.pkl"
    
    # Verify .hk directory was created with vendored repo
    [ -d .hk/vendors ]
    [ -d .hk/vendors/bufbuild-buf ]
    [ -f .hk/vendors/bufbuild-buf/.pre-commit-hooks.yaml ]
    
    # Verify .git directory was removed
    [ ! -d .hk/vendors/bufbuild-buf/.git ]
    
    # Verify go.mod exists (buf is a Go project)
    [ -f .hk/vendors/bufbuild-buf/go.mod ]
    
    # Verify hk.pkl references vendored hooks
    run cat hk.pkl
    assert_output --partial 'import ".hk/vendors/bufbuild-buf/hooks.pkl"'
    assert_output --partial "buf-format"
    assert_output --partial "buf-lint"
    assert_output --partial "Builtins.prettier"
    
    # Verify vendored PKL file was created
    [ -f .hk/vendors/bufbuild-buf/hooks.pkl ]
    
    # Verify the generated PKL file has correct structure for language: golang
    run cat .hk/vendors/bufbuild-buf/hooks.pkl
    assert_output --partial "buf_format"
    assert_output --partial "buf_lint"
    assert_output --partial ".gopath/bin"
    assert_output --partial "go install ./..."
    assert_output --partial "export GOPATH"
    
    # Verify hooks use the vendored Go binaries and are in linters
    run cat hk.pkl
    assert_output --partial "local linters"
    assert_output --partial "Vendors_bufbuild_buf.buf_format"
    assert_output --partial "Vendors_bufbuild_buf.buf_lint"
    refute_output --partial "custom_steps"
    
    # Note: buf uses language: golang so it will install via go install
    # The actual buf commands would require Go and proto files to test
}

@test "migrate precommit - vendor ruby hooks (jumanjihouse)" {
    cat <<PRECOMMIT > .pre-commit-config.yaml
repos:
-   repo: https://github.com/jumanjihouse/pre-commit-hooks
    rev: 3.0.0
    hooks:
    -   id: reek
-   repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v4.0.0
    hooks:
    -   id: prettier
PRECOMMIT

    run hk migrate pre-commit
    assert_success
    assert_output --partial "Successfully migrated to hk.pkl"
    
    # Verify .hk directory was created with vendored repo
    [ -d .hk/vendors ]
    [ -d .hk/vendors/jumanjihouse-pre-commit-hooks ]
    [ -f .hk/vendors/jumanjihouse-pre-commit-hooks/.pre-commit-hooks.yaml ]
    
    # Verify .git directory was removed
    [ ! -d .hk/vendors/jumanjihouse-pre-commit-hooks/.git ]
    
    # Verify gemspec exists (jumanjihouse is a Ruby project)
    [ -f .hk/vendors/jumanjihouse-pre-commit-hooks/fake_gem__.gemspec ]
    
    # Verify hk.pkl references vendored hooks
    run cat hk.pkl
    assert_output --partial 'import ".hk/vendors/jumanjihouse-pre-commit-hooks/hooks.pkl"'
    assert_output --partial "reek"
    assert_output --partial "Builtins.prettier"
    
    # Verify vendored PKL file was created
    [ -f .hk/vendors/jumanjihouse-pre-commit-hooks/hooks.pkl ]
    
    # Verify the generated PKL file has correct structure for language: ruby
    run cat .hk/vendors/jumanjihouse-pre-commit-hooks/hooks.pkl
    assert_output --partial "reek ="
    assert_output --partial ".gem-home/bin"
    assert_output --partial "gem build"
    assert_output --partial "gem install"
    assert_output --partial "GEM_HOME"
    assert_output --partial "PATH="
    
    # Verify hooks use the vendored Ruby binaries and are in linters
    run cat hk.pkl
    assert_output --partial "local linters"
    assert_output --partial "Vendors_jumanjihouse_pre_commit_hooks.reek"
    refute_output --partial "custom_steps"
    
    # Create a test Ruby file with code smells
    mkdir -p test
    cat <<RUBY > test/example.rb
class BadCode
  def smelly_method
    x = 1
    y = 2
    z = 3
    return x + y + z
  end
end
RUBY
    
    # Run reek hook
    run hk check test/example.rb
    assert_failure
    assert_output --partial "reek"
    assert_output --partial "IrresponsibleModule"
    assert_output --partial "UncommunicativeVariableName"
}
