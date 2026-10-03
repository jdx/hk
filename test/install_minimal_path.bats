#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] { steps { ["a"] { check = "true" } } }
}
EOF
    HK_DIR="$(dirname "$(command -v hk)")"
}

teardown() {
    _common_teardown
}

@test "legacy shim finds hk when Git runs it with a minimal PATH" {
    hk install --legacy
    run cat .git/hooks/pre-commit
    assert_output --partial 'PATH="$PATH":'
    assert_output --partial 'exec hk run pre-commit --from-hook'

    run env -i PATH=/usr/bin:/bin .git/hooks/pre-commit
    assert_success
}

@test "legacy shim prefers an hk already on PATH over the fallback directory" {
    hk install --legacy
    mkdir fake
    printf '#!/bin/sh\necho fake-hk-ran\n' > fake/hk
    chmod +x fake/hk

    run env -i PATH="$PWD/fake:/usr/bin:/bin" .git/hooks/pre-commit
    assert_output --partial "fake-hk-ran"
}

@test "legacy shim still fails closed when hk cannot be found anywhere" {
    hk install --legacy
    # Point the fallback at a directory without hk.
    sed -i.bak "s|PATH=\"\$PATH\":[^;]*;|PATH=\"\$PATH\":/nonexistent;|" .git/hooks/pre-commit

    run -127 env -i PATH=/usr/bin:/bin .git/hooks/pre-commit
}

@test "local config hook finds hk with a minimal PATH" {
    git_version="$(git --version | awk '{print $3}')"
    if ! printf '2.54\n%s\n' "$git_version" | sort -V -C; then
        skip "needs Git 2.54+ for config-based hooks"
    fi
    hk install
    run git config --local --get hook.hk-pre-commit.command
    assert_output --partial 'PATH="$PATH":'
    command="$output"

    run env -i PATH=/usr/bin:/bin sh -c "$command" hook-shim
    assert_success
}
