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
                    stage = "e.txt"
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
