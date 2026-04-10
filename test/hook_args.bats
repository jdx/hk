#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "hook_args is empty for hooks without a dedicated handler" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["post-applypatch"] {
        steps {
            ["capture"] { check = "echo '{{ hook_args }}' > hook_args.txt" }
        }
    }
}
EOF
    echo "a" > a.txt && git add a.txt && git commit -m "init"
    run hk run post-applypatch
    assert_success
    run cat hook_args.txt
    assert_output ""
}
