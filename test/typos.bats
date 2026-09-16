#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
steps { ["typos"] = Builtins.typos }
PKL
}

teardown() {
    _common_teardown
}

@test "typos builtin check and fix tests" {
    run hk test --step typos
    assert_success
    assert_output --partial "ok - typos :: check respects config exclude"
    assert_output --partial "ok - typos :: fix bad file"
}

@test "typos preserves silent operational failures" {
    mkdir bin
    cat > bin/typos <<'SH'
#!/bin/sh
printf 'configuration could not be read\n' >&2
exit 78
SH
    chmod +x bin/typos
    export PATH="$PWD/bin:$PATH"
    printf 'correct\n' > test.txt
    for shell in "sh -o errexit -c" "sh -c"; do
        cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
steps { ["typos"] = (Builtins.typos) { shell = "$shell" } }
PKL
        run hk check --check test.txt
        assert_failure 78
        assert_output --partial "configuration could not be read"
        run hk fix --no-stage test.txt
        assert_failure 78
    done
}

@test "typos rejects invalid native configuration" {
    printf 'correct\n' > test.txt
    printf '[invalid TOML\n' > typos.toml
    run hk check --check test.txt
    assert_failure
    run hk fix --no-stage test.txt
    assert_failure
}
