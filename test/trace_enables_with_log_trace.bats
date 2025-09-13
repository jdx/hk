#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat >hk.toml <<EOF
[[hook.check.steps]]
inline = 'echo "test check"'
EOF
    git add .
    git commit -m "initial"
}

@test "trace: enabled when HK_LOG=trace" {
    HK_LOG=trace run hk check
    assert_success
    # pretty tracing output should include span names
    assert_output --partial "config.load"
    assert_output --partial "hook.run"
}

@test "trace: enabled when -vv (trace level)" {
    run hk -vv check
    assert_success
    assert_output --partial "config.load"
    assert_output --partial "hook.run"
}
