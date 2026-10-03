setup() {
    load 'test_helper/common_setup'
    _common_setup
}

export BATS_TEST_TIMEOUT="${BATS_TEST_TIMEOUT:-10}"

teardown() {
    _common_teardown
}

# `timeout` is not on macOS; perl's alarm kills a hang with SIGALRM (status 142).
_timeout() {
    perl -e 'alarm shift; exec @ARGV' "$@"
}

@test "depends" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["a"] { fix = "echo ITWORKS > a.txt" }
            ["b"] { fix = "cat a.txt > b.txt"; depends = List("a") }
            ["c"] { fix = "cat b.txt > c.txt"; depends = List("b") }
            ["d"] { fix = "cat c.txt > d.txt"; depends = List("c") }
            ["e"] { depends = List("d")
                    check = """
if [ \$(cat d.txt) = "ITWORKS" ]; then
    exit 0
fi
echo "d.txt does not contain ITWORKS"
exit 1
""" }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"
    hk fix -v
}

@test "file depends" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["a"] { fix = "echo ITWORKS > a.txt"; stage = "a.txt" }
            ["b"] { fix = "cat a.txt > b.txt"; depends = List("a"); stage = "b.txt"; glob = "a.txt" }
            ["c"] { fix = "cat b.txt > c.txt"; depends = List("b"); stage = "c.txt"; glob = "b.txt" }
            ["d"] { fix = "cat c.txt > d.txt"; depends = List("c"); stage = "d.txt"; glob = "c.txt" }
            ["e"] { depends = List("d")
                    glob = "d.txt"
                    check = """
if [ \$(cat d.txt) = "ITWORKS" ]; then
    exit 0
fi
echo "d.txt does not contain ITWORKS"
exit 1
""" }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"
    hk fix -v
}

@test "dependent step proceeds when dependency fails and fail_fast is false" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
    ["check"] {
        steps {
            ["fail"] {
                glob = "**/*"
                check = "echo FAIL && exit 1"
            }
            ["should-pass"] {
                glob = "**/*"
                depends = List("fail")
                check = "echo SHOULD_PASS"
            }
        }
    }
}
EOF
    echo "test" > test.txt
    git add hk.pkl test.txt
    git commit -m "initial commit"

    run hk check --all
    assert_failure
    assert_output --partial "FAIL"
    assert_output --partial "SHOULD_PASS"
}

@test "dependent step does not proceed when dependency fails and fail_fast is true" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
    ["check"] {
        steps {
            ["fail"] {
                glob = "**/*"
                check = "echo FAIL && exit 1"
            }
            ["should-not-pass"] {
                glob = "**/*"
                depends = List("fail")
                check = "echo SHOULD_NOT_PASS"
            }
        }
    }
}
EOF
    echo "test" > test.txt
    git add hk.pkl test.txt
    git commit -m "initial commit"

    run hk check --all
    assert_failure
    assert_output --partial "FAIL"
    refute_output --partial "SHOULD_NOT_PASS"
}

@test "dependent step runs when a workspace_indicator dependency finds no workspace" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["workspace"] {
                glob = "**/*.txt"
                workspace_indicator = "marker"
                check = "echo WORKSPACE_RAN"
            }
            ["after"] {
                glob = "**/*.txt"
                depends = List("workspace")
                check = "echo AFTER_RAN"
            }
        }
    }
}
EOF
    echo "test" > test.txt
    git add hk.pkl test.txt
    git commit -m "initial commit"

    # No `marker` file exists in any ancestor of test.txt, so `workspace` has no jobs.
    # `timeout` makes a regression fail instead of hanging on the dependency.
    run _timeout 8 hk check --all
    assert_success
    refute_output --partial "WORKSPACE_RAN"
    assert_output --partial "AFTER_RAN"
}

@test "dependent step runs when an unfiltered batch dependency has no files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["batched"] {
                batch = true
                check = "echo BATCHED_RAN"
            }
            ["after"] {
                depends = List("batched")
                check = "echo AFTER_RAN"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Clean tree: nothing is staged or changed, so the batch step has no jobs.
    # `timeout` makes a regression fail instead of hanging on the dependency.
    run _timeout 8 hk check
    assert_success
    assert_output --partial "AFTER_RAN"
}
