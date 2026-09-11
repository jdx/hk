#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

_add_gitlink() {
    git update-index --add --cacheinfo "160000,1111111111111111111111111111111111111111,$1"
}

@test "util forbid-submodules - detects a submodule" {
    echo "# Project" > README.md
    git add -A
    _add_gitlink vendor/dep

    run hk util forbid-submodules
    assert_failure
    assert_output --partial "vendor/dep"
    assert_output --partial "git rm <submodule>"
}

@test "util forbid-submodules - passes without submodules" {
    echo "# Project" > README.md
    git add -A

    run hk util forbid-submodules
    assert_success
    refute_output
}

@test "util forbid-submodules - passes on an empty repository" {
    run hk util forbid-submodules
    assert_success
    refute_output
}

@test "util forbid-submodules - detects multiple submodules" {
    echo "# Project" > README.md
    git add -A
    _add_gitlink vendor/dep
    _add_gitlink "third party/other"

    run hk util forbid-submodules
    assert_failure
    assert_output --partial "vendor/dep"
    assert_output --partial "third party/other"
}

@test "util forbid-submodules - ignores .gitmodules without a gitlink" {
    printf '[submodule "dep"]\n\tpath = vendor/dep\n' > .gitmodules
    git add -A

    run hk util forbid-submodules
    assert_success
    refute_output
}

@test "util forbid-submodules - scopes to the supplied paths" {
    echo "# Project" > README.md
    git add -A
    _add_gitlink vendor/dep

    run hk util forbid-submodules README.md
    assert_success
    refute_output

    run hk util forbid-submodules vendor
    assert_failure
    assert_output --partial "vendor/dep"
}

@test "util forbid-submodules - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["forbid-submodules"] = Builtins.forbid_submodules
        }
    }
}
HK

    echo "# Project" > README.md
    git add -A
    _add_gitlink vendor/dep

    run hk check --all
    assert_failure
    assert_output --partial "vendor/dep"
}
