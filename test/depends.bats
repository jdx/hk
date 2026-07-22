setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
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

    run timeout 5s hk check --all
    assert_failure
    refute [ "$status" -eq 124 ]
    assert_output --partial "FAIL"
    assert_output --partial "SHOULD_PASS"
}
