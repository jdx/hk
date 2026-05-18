#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

require_git_2_54() {
    local version major minor
    version="$(git version | awk '{print $3}')"
    major="${version%%.*}"
    minor="${version#*.}"
    minor="${minor%%.*}"
    if [ "$major" -lt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -lt 54 ]; }; then
        skip "hk install --global requires Git 2.54+"
    fi
}

@test "hk install --global writes an absolute hk command" {
    require_git_2_54

    run hk install --global
    assert_success

    run git config --global --get hook.hk-pre-commit.command
    assert_success
    assert_output --partial 'run pre-commit --from-hook "$@"'
    refute_output --partial '|| hk run'

    run git config --global --get hook.hk-pre-rebase.command
    assert_failure

    mkdir "$TEST_TEMP_DIR/src/without-hk"
    cd "$TEST_TEMP_DIR/src/without-hk"
    git init .

    command="$(git config --global --get hook.hk-pre-commit.command)"
    run env PATH="/usr/bin:/bin:/usr/sbin:/sbin" sh -c "$command" hook-shim
    assert_success
}

@test "hk install --global uses project hooks when hk config exists" {
    require_git_2_54

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-rebase"] {
        steps {
            ["noop"] { check = "exit 0" }
        }
    }
}
EOF

    run hk install --global
    assert_success

    run git config --global --get hook.hk-pre-rebase.command
    assert_success
    assert_output --partial 'run pre-rebase --from-hook "$@"'

    run git config --global --get hook.hk-pre-commit.command
    assert_failure
}

@test "hk install --global --mise writes home-relative mise with explicit hk tool" {
    require_git_2_54

    mkdir "$TEST_TEMP_DIR/bin"
    cat >"$TEST_TEMP_DIR/bin/mise" <<'EOF'
#!/bin/sh
exit 0
EOF
    chmod +x "$TEST_TEMP_DIR/bin/mise"
    PATH="$TEST_TEMP_DIR/bin:$PATH"

    run hk install --global --mise
    assert_success

    run git config --global --get hook.hk-pre-commit.command
    assert_success
    assert_output --partial "~/bin/mise x hk -- hk run pre-commit --from-hook"
    refute_output --partial '|| mise x -- hk run'

    command="$(git config --global --get hook.hk-pre-commit.command)"
    run env PATH="/usr/bin:/bin:/usr/sbin:/sbin" sh -c "$command" hook-shim
    assert_success
}
