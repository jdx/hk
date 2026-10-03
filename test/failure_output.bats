#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "a hook error is printed once and without a Rust source location" {
    touch a.txt
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.txt"
                check = "echo {{ nope_var }}"
            }
        }
    }
}
EOF
    git add .

    run hk check --all
    assert_failure
    assert_output --partial "Variable \`nope_var\` is not defined"
    refute_output --partial "Location:"
    # The cause chain appears once, not once from the logger and again from main.
    [ "$(echo "$output" | grep -c 'Variable `nope_var` is not defined')" -eq 1 ]
}

@test "a command error keeps its Rust source location with -v" {
    touch a.txt
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.txt"
                check = "echo {{ nope_var }}"
            }
        }
    }
}
EOF
    git add .

    run hk -v check --all
    assert_failure
    assert_output --partial "Location:"
}

@test "a missing tool in a check does not suggest running the fixer" {
    touch a.txt
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["missing"] {
                glob = "*.txt"
                check = "hk-no-such-tool-xyz {{files}}"
                fix = "hk-no-such-tool-xyz --fix {{files}}"
            }
        }
    }
}
EOF
    git add .

    run hk check --all
    assert_failure
    # The non-TTY end-of-run failure summary is kept (#890).
    assert_output --partial "missing stderr:"
    assert_output --regexp "hk-no-such-tool-xyz: .*not found"
    refute_output --partial "To fix, run"
}

@test "a failing check still suggests running the fixer" {
    touch a.txt
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.txt"
                check = "exit 1"
                fix = "true"
            }
        }
    }
}
EOF
    git add .

    run hk check --all
    assert_failure
    assert_output --partial "To fix, run"
}

@test "commit-msg explains the expected format for a title without a type" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["commit-msg"] {
        steps {
            ["cc"] { check = "hk util check-conventional-commit {{commit_msg_file}}" }
        }
    }
}
EOF
    echo "wip stuff" > msg

    run hk run commit-msg msg
    assert_failure
    assert_output --partial "Invalid commit type: 'wip stuff'"
    assert_output --partial "<type>(<scope>): <description>"
    assert_output --partial "Allowed types:"
}
