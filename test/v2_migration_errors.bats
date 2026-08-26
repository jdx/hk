#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

assert_raw_config_removed() {
    local filename="$1"
    echo '{}' > "$filename"

    run hk check --all
    assert_failure
    assert_output --partial "configuration was removed in hk v2"
    assert_output --partial "$filename"
    assert_output --partial "to hk.pkl"
}

@test "hk.toml is rejected with migration guidance" {
    assert_raw_config_removed hk.toml
}

@test "hk.yaml is rejected with migration guidance" {
    assert_raw_config_removed hk.yaml
}

@test "hk.yml is rejected with migration guidance" {
    assert_raw_config_removed hk.yml
}

@test "hk.json is rejected with migration guidance" {
    assert_raw_config_removed hk.json
}

@test "hk generate is rejected with migration guidance" {
    run hk generate
    assert_failure
    assert_output --partial '`hk generate` was removed in hk v2'
    assert_output --partial '`hk init`'
}

@test "removed byte-order-marker aliases have migration guidance" {
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
steps { ["bom"] = Builtins.check_byte_order_marker() }
EOF

    run hk validate
    assert_failure
    assert_output --partial "Builtins.check_byte_order_marker() was removed in hk v2"
    assert_output --partial "Builtins.byte_order_marker()"
}

@test "removed fix byte-order-marker alias has migration guidance" {
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
steps { ["bom"] = Builtins.fix_byte_order_marker() }
EOF

    run hk validate
    assert_failure
    assert_output --partial "Builtins.fix_byte_order_marker() was removed in hk v2"
    assert_output --partial "Builtins.byte_order_marker()"
}
