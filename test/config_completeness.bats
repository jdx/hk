#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "Ensure settings.toml and Config.pkl match" {
    # Checks to see if the root properties in Config.pkl are all defined in settings.toml.
    # Excludes: hooks, output, and min_hk_version
    diff -u --label "settings.toml" --label "Config.pkl" \
        <(taplo get -f $PROJECT_ROOT/settings.toml -o json | jq -r 'to_entries | map(select(.value.sources | has("pkl"))) | .[].key' | sort)\
        <(pkl eval $PKL_PATH/Config.pkl --format json -x "import (\"file:$PROJECT_ROOT/scripts/reflect.pkl\").render(module)" | jq -r '.moduleClass.properties | keys[] | select(. != "hooks" and . != "output" and . != "min_hk_version")' | sort)
}
