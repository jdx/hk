#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

_write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["trivy"] { check = "echo ran-trivy" }
            ["prettier"] { check = "echo ran-prettier" }
            ["lint-group"] = new Group {
                steps {
                    ["inner-lint"] { check = "echo ran-inner" }
                }
            }
        }
    }
    ["fix"] {
        steps {
            ["trivy"] { fix = "echo fixed-trivy" }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m init
}

@test "check --step with an unknown name fails with a suggestion" {
    _write_config
    run hk check --step trial --all
    assert_failure
    assert_output --partial "unknown step 'trial' for hook 'check'. Did you mean 'trivy'?"
    assert_output --partial "Available steps: trivy, prettier, inner-lint"
    refute_output --partial "no steps to run"
}

@test "check --step with one known and one unknown name fails without running either" {
    _write_config
    run hk check --step trivy --step prettir --all
    assert_failure
    assert_output --partial "unknown step 'prettir'"
    assert_output --partial "Did you mean 'prettier'?"
    refute_output --partial "ran-trivy"
}

@test "check --step accepts a step inside a group" {
    _write_config
    run hk check --step inner-lint --all
    assert_success
    assert_output --partial "ran-inner"
    refute_output --partial "ran-trivy"
}

@test "check --step with a group name points at the steps inside it" {
    _write_config
    run hk check --step lint-group --all
    assert_failure
    assert_output --partial "'lint-group' is a group; name its steps instead: inner-lint."
}

@test "fix and run reject unknown --step names too" {
    _write_config
    run hk fix --step trivi --all
    assert_failure
    assert_output --partial "unknown step 'trivi' for hook 'fix'. Did you mean 'trivy'?"
    run hk run check --step trivi --all
    assert_failure
    assert_output --partial "unknown step 'trivi' for hook 'check'"
}

@test "an unknown --step name is rejected under --plan" {
    _write_config
    run hk check --plan --step trivi --all
    assert_failure
    assert_output --partial "unknown step 'trivi'"
}

@test "an unknown --skip-step name only warns" {
    _write_config
    run hk check --skip-step prettir --all
    assert_success
    assert_output --partial "--skip-step prettir: no such step in hook 'check'. Did you mean 'prettier'?"
    assert_output --partial "ran-trivy"
}

@test "--step finds a subproject step by its subdir-prefixed name" {
    mkdir -p pkg
    cat <<EOF > pkg/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["lint"] { check = "echo ran-pkg-lint" } } } }
EOF
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("pkg")
EOF
    git add .
    git commit -m init
    run hk check --step pkg:lint --all
    assert_success
    assert_output --partial "ran-pkg-lint"
    run hk check --step lint --all
    assert_failure
    assert_output --partial "unknown step 'lint' for hook 'check'. Did you mean 'pkg:lint'?"
}

@test "--plan also warns about an unknown --skip-step name" {
    _write_config
    run hk check --plan --skip-step prettir --all
    assert_success
    assert_output --partial "--skip-step prettir: no such step in hook 'check'. Did you mean 'prettier'?"
}
