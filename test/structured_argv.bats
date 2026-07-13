#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
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
