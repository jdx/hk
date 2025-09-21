#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "hk check fails on invalid config" {
    cd "$BATS_TEST_TMPDIR"

    # Create invalid pkl config
    cat > hk.pkl <<'EOF'
invalid pkl syntax content
EOF

    # hk check should fail with config error
    run hk check
    assert_failure
    assert_output --partial "Failed to read config file"
    assert_output --partial "Invalid property definition"
}

@test "hk fix fails on invalid config" {
    cd "$BATS_TEST_TMPDIR"

    # Create invalid pkl config
    cat > hk.pkl <<'EOF'
this is not valid pkl
EOF

    # hk fix should fail with config error
    run hk fix
    assert_failure
    assert_output --partial "Failed to read config file"
    assert_output --partial "Pkl Error"
}

@test "hk run fails on invalid config" {
    cd "$BATS_TEST_TMPDIR"

    # Create invalid pkl config
    cat > hk.pkl <<'EOF'
broken config
EOF

    # hk run should fail with config error
    run hk run pre-commit
    assert_failure
    assert_output --partial "Failed to read config file"
    assert_output --partial "Invalid property definition"
}

@test "hk config commands fail on invalid config" {
    cd "$BATS_TEST_TMPDIR"

    # Create invalid pkl config
    cat > hk.pkl <<'EOF'
not pkl format
EOF

    # config get should fail
    run hk config get jobs
    assert_failure
    assert_output --partial "Failed to read config file"

    # config dump should fail
    run hk config dump
    assert_failure
    assert_output --partial "Failed to read config file"

    # config explain should fail
    run hk config explain jobs
    assert_failure
    assert_output --partial "Failed to read config file"
}


@test "config error shows helpful details" {
    cd "$BATS_TEST_TMPDIR"

    # Create config with specific syntax error
    cat > hk.pkl <<'EOF'
amends "pkl/Config.pkl"
invalid_field =
EOF

    # Should show line number and specific error
    run hk check
    assert_failure
    assert_output --partial "line 2"
    assert_output --partial "invalid_field"
}
