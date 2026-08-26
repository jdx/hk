#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

write_config() {
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

steps {
    ["shared"] {
        check = "echo inherited"
        fix = "echo fixed"
    }
}
EOF
}

@test "top-level steps create check fix and pre-commit hooks" {
    write_config

    run hk check --all
    assert_success
    assert_output --partial "inherited"

    run hk fix --all
    assert_success
    assert_output --partial "fixed"

    run hk run pre-commit --all --no-stage
    assert_success
    assert_output --partial "fixed"

    run hk run pre-push --all
    assert_failure
}

@test "explicit hook steps override inherited steps" {
    write_config
    cat >> hk.pkl <<'EOF'
hooks {
    ["check"] {
        steps {
            ["shared"] {
                check = "echo explicit"
            }
        }
    }
}
EOF

    run hk check --all
    assert_success
    assert_output --partial "explicit"
    refute_output --partial "inherited"
}

@test "disabled implicit hook is a successful no-op" {
    write_config
    cat >> hk.pkl <<'EOF'
hooks {
    ["check"] {
        enabled = false
    }
}
EOF

    run hk check --all
    assert_success
    refute_output --partial "inherited"
}
