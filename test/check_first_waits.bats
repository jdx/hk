#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "check_first waits" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        stash = "git"
        fix = true
        steps {
            ["a"] {
                glob = List("*.sh")
                stage = "*"
                check_first = true
                check = "echo 'start a' && sleep 0.1 && echo 'exit a' && exit 1"
                fix = "echo 'start a' && sleep 0.1 && echo 'end a'"
            }
            ["b"] {
                glob = List("*.sh")
                stage = "*"
                check_first = true
                check = "echo 'start b' && echo 'exit b' && exit 1"
                fix = "echo 'start b' && echo 'end b' > test.sh && echo 'end b'"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    touch test.sh
    git add test.sh
    run hk run pre-commit
    assert_success

    # runs b to completion without a
    assert_output --partial "  b – start b
  b – end b"
}
