#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "validation: check and check_diff are mutually exclusive" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = List("*.txt")
                check = "mock-linter"
                check_diff = "mock-linter --diff"
            }
        }
    }
}
EOF
    git add -A
    git commit -m "init"

    echo "hello" > test.txt
    git add test.txt

    run hk check
    assert_failure
    assert_output --partial "mutually exclusive"
}

@test "validation: check and check_diff are mutually exclusive in groups" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["my_group"] = new Group {
                steps {
                    ["lint"] {
                        glob = List("*.txt")
                        check = "mock-linter"
                        check_diff = "mock-linter --diff"
                    }
                }
            }
        }
    }
}
EOF
    git add -A
    git commit -m "init"

    echo "hello" > test.txt
    git add test.txt

    run hk check
    assert_failure
    assert_output --partial "mutually exclusive"
}
