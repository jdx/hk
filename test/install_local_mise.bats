#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# A stand-in mise (`x -- CMD...` runs CMD; anything else proves mise
# resolved) and a stand-in hk that calls `mise` the way a step does.
_stub_mise_and_hk() {
    mkdir "$TEST_TEMP_DIR/bin" "$TEST_TEMP_DIR/hkbin"
    cat >"$TEST_TEMP_DIR/bin/mise" <<'EOF'
#!/bin/sh
if [ "$1" = "x" ]; then
    while [ "$1" != "--" ]; do shift; done
    shift
    exec "$@"
fi
echo "mise reachable"
EOF
    cat >"$TEST_TEMP_DIR/hkbin/hk" <<'EOF'
#!/bin/sh
exec mise run hello
EOF
    chmod +x "$TEST_TEMP_DIR/bin/mise" "$TEST_TEMP_DIR/hkbin/hk"
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["noop"] { check = "true" } } } }
EOF
}

@test "hk install --mise launcher starts without mise on PATH" {
    if ! git version | awk '{split($3,v,"."); exit !(v[1]>2 || (v[1]==2 && v[2]>=54))}'; then
        skip "git 2.54+ required for config-based hooks"
    fi
    _stub_mise_and_hk
    PATH="$TEST_TEMP_DIR/bin:$PATH" run hk install --mise
    assert_success

    command="$(git config --local --get hook.hk-pre-commit.command)"
    # Run the launcher as Git would, with a minimal PATH that lacks mise.
    run env PATH="$TEST_TEMP_DIR/hkbin" /bin/sh -c "$command" hook-shim
    assert_success
    assert_output --partial "mise reachable"
}

@test "hk install --mise --legacy shim starts without mise on PATH" {
    _stub_mise_and_hk
    PATH="$TEST_TEMP_DIR/bin:$PATH" run hk install --mise --legacy
    assert_success

    run env PATH="$TEST_TEMP_DIR/hkbin" "$(git rev-parse --git-path hooks/pre-commit)"
    assert_success
    assert_output --partial "mise reachable"
}
