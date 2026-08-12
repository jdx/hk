#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "structured argv supports a literal argv prefix" {
    cat <<'EOF' > capture-prefix
#!/bin/sh
printf '%s\n' "$@" > argv.log
EOF
    chmod +x capture-prefix
    touch 'a b.txt' 'semi;colon.txt'
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["capture"] {
                glob = "*.txt"
                prefix = List("{{root}}/capture-prefix", "prefix value", "\$HOME", "*")
                check = new Command {
                    argv = List("tool", "--flag", "{{files}}")
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_success

    run cat argv.log
    assert_success
    assert_line --index 0 'prefix value'
    assert_line --index 1 '$HOME'
    assert_line --index 2 '*'
    assert_line --index 3 'tool'
    assert_line --index 4 '--flag'
    assert_line 'a b.txt'
    assert_line 'semi;colon.txt'
    assert_equal "${#lines[@]}" 7
}

@test "structured argv preserves literal arguments and file boundaries" {
    cat <<'EOF' > capture-args
#!/bin/sh
printf '%s\n' "$@" > argv.log
EOF
    chmod +x capture-args
    touch 'a b.txt' 'semi;colon.txt'
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["capture"] {
                glob = "*.txt"
                check = new Command {
                    argv = List("{{root}}/capture-args", "\$HOME", "*", "{{files}}")
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_success

    run cat argv.log
    assert_success
    assert_line --index 0 '$HOME'
    assert_line --index 1 '*'
    assert_line 'a b.txt'
    assert_line 'semi;colon.txt'
    assert_equal "${#lines[@]}" 4
}

@test "structured argv prefix composes with builtins" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["ruff"] = (Builtins.ruff) {
                prefix = List("mise", "x", "--")
            }
            ["ruff_format"] = (Builtins.ruff_format) {
                prefix = List("mise", "x", "--")
            }
        }
    }
}
EOF

    run hk validate
    assert_success
}
