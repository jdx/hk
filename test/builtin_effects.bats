#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["trailing_whitespace"] = Builtins.trailing_whitespace()
            ["vale"] = Builtins.vale()
        }
    }
}
EOF
    touch clean.txt
    git add .
    git commit -m init
}

@test "builtin plans expose read, write, and fix effects" {
    write_config

    run bash -c "hk check --all --plan --json 2>/dev/null"
    assert_success
    run jq -r '.steps[] | [.name, .metadata.effect] | @tsv' <<<"$output"
    assert_success
    assert_output $'trailing_whitespace\tread\nvale\twrite'

    run bash -c "hk check --fix --all --plan --json 2>/dev/null"
    assert_success
    run jq -r '.steps[] | select(.name == "trailing_whitespace") | .metadata.effect' <<<"$output"
    assert_success
    assert_output "write"
}

@test "safe mode executes a builtin read command" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["trailing_whitespace"] = Builtins.trailing_whitespace()
        }
    }
}
EOF
    touch clean.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_success
}

@test "overriding a builtin command drops its inherited effect" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["overridden"] = (Builtins.trailing_whitespace()) {
                check_diff = "touch should-not-run"
            }
        }
    }
}
EOF
    touch clean.txt
    git add .
    git commit -m init

    run hk check --all --safe
    assert_failure
    assert_output --partial "overridden.check_diff: effect is unknown"
    assert_file_not_exists should-not-run
}
