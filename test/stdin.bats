#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "stdin and interactive errors" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["step"] {
                stdin = "{{ files_list }}"
                interactive = true
            }
        }
    }
}
EOF
    echo "content" > file.txt

    run hk check
    assert_failure
    assert_output --partial "Step 'step' can't have both \`stdin\` and \`interactive = true\`."
}

@test "stdin works with xargs" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["ruff-format"] {
                stdin = "{{ files_list | join(sep='\\n') }}"
                fix = "xargs ruff format"
            }
        }
    }
}
EOF
    # NB: Use `.txt` to be sure ruff didn't "find" a `.py` file.
    echo "x=1" > file.txt

    run hk fix file.txt
    assert_success

    run cat file.txt
    assert_success
    assert_output "x = 1"
}

@test "stdin works with xargs as prefix" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["ruff-format"] {
                stdin = "{{ files_list | join(sep='\\n') }}"
                prefix = "xargs"
                fix = "ruff format"
            }
        }
    }
}
EOF
    # NB: Use `.txt` to be sure ruff didn't "find" a `.py` file.
    echo "x=1" > file.txt

    run hk fix file.txt
    assert_success

    run cat file.txt
    assert_success
    assert_output "x = 1"
}

@test "stdin works with ruff atfiles /dev/stdin" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["ruff-format"] {
                stdin = "{{ files_list | join(sep='\\n') }}"
                fix = "ruff format @/dev/stdin"
            }
        }
    }
}
EOF
    # NB: Use `.txt` to be sure ruff didn't "find" a `.py` file.
    echo "x=1" > file.txt

    run hk fix file.txt
    assert_success

    run cat file.txt
    assert_success
    assert_output "x = 1"
}
