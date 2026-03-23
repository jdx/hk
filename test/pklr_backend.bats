#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_PKL_BACKEND=pklr
}

teardown() {
    _common_teardown
}

@test "pklr backend can evaluate a basic config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["echo"] {
                check = "echo ok"
            }
        }
    }
}
EOF

    run hk check --all
    assert_success
}

@test "pklr backend validates config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] { check = "echo step1" }
        }
    }
}
EOF

    run hk validate
    assert_success
}
