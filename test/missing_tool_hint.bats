#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "a builtin step with a missing shell tool suggests mise use" {
    echo 'echo hi' > a.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["shfmt"] {
                glob = "*.sh"
                check = "hk-no-such-tool-xyz {{files}}"
            }
        }
    }
}
EOF
    run hk check --all
    assert_failure
    assert_output --partial "hint: shfmt could not run because a tool it needs is not installed"
    assert_output --partial "mise use shfmt"
}

@test "a missing argv program named after a builtin suggests its mise tools" {
    echo 'x = 1' > a.py
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.py"
                check = new Command { argv = List("ruff", "check", "{{files}}") }
            }
        }
    }
}
EOF
    PATH="$(dirname "$(command -v hk)"):$(dirname "$(command -v git)"):/usr/bin:/bin"
    if command -v ruff >/dev/null; then
        skip "ruff is installed"
    fi
    run hk check --all
    assert_failure
    assert_output --partial "ruff: command not found"
    assert_output --partial "mise use ruff"
}

@test "a missing tool in a custom step gives no mise hint" {
    touch a.txt
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["custom"] {
                glob = "*.txt"
                check = "hk-no-such-tool-xyz {{files}}"
            }
        }
    }
}
EOF
    run hk check --all
    assert_failure
    refute_output --partial "mise use"
}
