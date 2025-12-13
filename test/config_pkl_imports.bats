#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# Ensure we only import stdlib modules
# (This is convenient for users who want to use `amends https://raw.githubusercontent.com/jdx/hk/refs/tags/v1.27.0/pkl/Config.pkl`)
@test "Config.pkl only imports stdlib" {
    # Parse the imports and check that all direct imports from Config.pkl start with "pkl:"
    run bash -c "pkl analyze imports --format json \"$PKL_PATH/Config.pkl\" | jq -r '.imports[\"file://$PKL_PATH/Config.pkl\"][] | .uri'"
    assert_success

    # Verify each import line starts with "pkl:" (stdlib)
    while IFS= read -r import_uri; do
        [[ "$import_uri" == pkl:* ]] || {
            echo "Non-stdlib import found: $import_uri"
            return 1
        }
    done <<< "$output"

    # Ensure we actually found at least one import
    assert_output --partial "pkl:"
}
