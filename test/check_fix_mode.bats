#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["probe"] {
                glob = List("*.txt")
                check = "echo RAN_CHECK"
                fix = "echo RAN_FIX"
            }
        }
    }
    ["check"] {
        steps {
            ["probe"] {
                glob = List("*.txt")
                check = "echo RAN_CHECK"
                fix = "echo RAN_FIX"
            }
        }
    }
}
EOF
    echo "hello" > a.txt
    git add -A
    git commit -q -m init
}

teardown() {
    _common_teardown
}

@test "hk fix runs fix commands by default" {
    run hk fix --all
    assert_success
    assert_output --partial "RAN_FIX"
    refute_output --partial "RAN_CHECK"
}

@test "HK_CHECK forces check commands" {
    HK_CHECK=1 run hk fix --all
    assert_success
    assert_output --partial "RAN_CHECK"
    refute_output --partial "RAN_FIX"
}

@test "HK_FIX=0 forces check commands" {
    HK_FIX=0 run hk fix --all
    assert_success
    assert_output --partial "RAN_CHECK"
    refute_output --partial "RAN_FIX"
}

@test "git config hk.check forces check commands" {
    git config --local hk.check true
    run hk fix --all
    assert_success
    assert_output --partial "RAN_CHECK"
    refute_output --partial "RAN_FIX"
}

@test "git config hk.fix=false forces check commands" {
    git config --local hk.fix false
    run hk fix --all
    assert_success
    assert_output --partial "RAN_CHECK"
    refute_output --partial "RAN_FIX"
}

@test "HK_CHECK=0 overrides git config hk.check" {
    git config --local hk.check true
    HK_CHECK=0 run hk fix --all
    assert_success
    assert_output --partial "RAN_FIX"
    refute_output --partial "RAN_CHECK"
}

@test "--fix overrides HK_CHECK" {
    HK_CHECK=1 run hk fix --all --fix
    assert_success
    assert_output --partial "RAN_FIX"
    refute_output --partial "RAN_CHECK"
}

@test "--fix overrides HK_FIX=0" {
    HK_FIX=0 run hk fix --all --fix
    assert_success
    assert_output --partial "RAN_FIX"
    refute_output --partial "RAN_CHECK"
}

@test "--check forces check commands" {
    run hk fix --all --check
    assert_success
    assert_output --partial "RAN_CHECK"
    refute_output --partial "RAN_FIX"
}

@test "hk check runs check commands and honors --fix" {
    run hk check --all
    assert_success
    assert_output --partial "RAN_CHECK"
    refute_output --partial "RAN_FIX"

    run hk check --all --fix
    assert_success
    assert_output --partial "RAN_FIX"
    refute_output --partial "RAN_CHECK"
}
