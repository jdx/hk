#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "condition" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["a"] { fix = "echo ITWORKS > a.txt"; condition = "true" }
            ["b"] { fix = "echo ITWORKS > b.txt"; condition = "false" }
            ["c"] { fix = "echo ITWORKS > c.txt"; condition = "exec('echo ITWORKS') == 'ITWORKS\n'" }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"
    hk fix -v
    assert_file_exists a.txt
    assert_file_not_exists b.txt
    assert_file_exists c.txt
}
