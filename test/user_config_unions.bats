#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "skip_steps from the project and the user config are unioned" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
skip_steps = List("a")
hooks {
    ["check"] {
        steps {
            ["a"] { check = "echo ran-a" }
            ["b"] { check = "echo ran-b" }
            ["c"] { check = "echo ran-c" }
        }
    }
}
EOF
    mkdir -p "$HOME/.config/hk"
    cat <<EOF > "$HOME/.config/hk/config.pkl"
amends "$PKL_PATH/Config.pkl"
skip_steps = List("b")
EOF
    run hk check --all
    assert_success
    assert_output --partial "ran-c"
    refute_output --partial "ran-a"
    refute_output --partial "ran-b"
}

@test "skip_hooks and hide_warnings from the project and the user config are unioned" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
skip_hooks = List("pre-push")
hide_warnings = List("missing-profiles")
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    mkdir -p "$HOME/.config/hk"
    cat <<EOF > "$HOME/.config/hk/config.pkl"
amends "$PKL_PATH/Config.pkl"
skip_hooks = List("pre-commit", "pre-push")
hide_warnings = List("other-warning")
EOF
    run hk config get skip_hooks
    assert_success
    assert_output --partial "pre-push"
    assert_output --partial "pre-commit"
    run hk config get hide_warnings
    assert_success
    assert_output --partial "missing-profiles"
    assert_output --partial "other-warning"
}

@test "a project list is kept when the user config sets none" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
skip_steps = List("a")
hooks { ["check"] { steps { ["a"] { check = "echo ran-a" } ["b"] { check = "echo ran-b" } } } }
EOF
    run hk check --all
    assert_success
    assert_output --partial "ran-b"
    refute_output --partial "ran-a"
}

@test "hk.local.pkl that does not amend hk.pkl warns that it replaces the shared config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["shared"] { check = "echo ran-shared" } } } }
EOF
    cat <<EOF > hk.local.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["local"] { check = "echo ran-local" } } } }
EOF
    run hk check --all
    assert_success
    assert_output --partial "hk.local.pkl does not amend"
    assert_output --partial "replaces the shared configuration"
    assert_output --partial "ran-local"
    refute_output --partial "ran-shared"
}

@test "the replaces-shared warning can be hidden" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["shared"] { check = "echo ran-shared" } } } }
EOF
    cat <<EOF > hk.local.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["local"] { check = "echo ran-local" } } } }
EOF
    HK_HIDE_WARNINGS=local-config-replaces-shared run hk check --all
    assert_success
    refute_output --partial "does not amend"
}

@test "hk.local.pkl that amends hk.pkl does not warn" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["shared"] { check = "echo ran-shared" } } } }
EOF
    cat <<EOF > hk.local.pkl
amends "./hk.pkl"
hooks { ["check"] { steps { ["local"] { check = "echo ran-local" } } } }
EOF
    run hk check --all
    assert_success
    refute_output --partial "does not amend"
    assert_output --partial "ran-shared"
    assert_output --partial "ran-local"
}

@test "hk.local.pkl without a shared hk.pkl does not warn" {
    cat <<EOF > hk.local.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["local"] { check = "echo ran-local" } } } }
EOF
    run hk check --all
    assert_success
    refute_output --partial "does not amend"
}

@test ".config/hk.local.pkl replacing a parent hk.pkl suggests the parent path" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["shared"] { check = "echo ran-shared" } } } }
EOF
    mkdir -p .config
    cat <<EOF > .config/hk.local.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["local"] { check = "echo ran-local" } } } }
EOF
    run hk check --all
    assert_success
    assert_output --partial 'Add `amends "../hk.pkl"`'
}

@test "hk.local.pkl that only imports hk.pkl still warns" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["shared"] { check = "echo ran-shared" } } } }
EOF
    cat <<EOF > hk.local.pkl
amends "$PKL_PATH/Config.pkl"
import "./hk.pkl" as Shared
hooks { ["check"] { steps { ["local"] { check = "echo ran-local" } } } }
EOF
    run hk check --all
    assert_success
    assert_output --partial "hk.local.pkl does not amend"
}

@test "a subproject hk.local.pkl that replaces its hk.pkl warns" {
    mkdir -p pkg
    cat <<EOF > pkg/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["shared"] { check = "echo ran-shared" } } } }
EOF
    cat <<EOF > pkg/hk.local.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["local"] { check = "echo ran-local" } } } }
EOF
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("pkg")
EOF
    run hk check --all
    assert_success
    assert_output --partial "pkg/hk.local.pkl does not amend"
}
