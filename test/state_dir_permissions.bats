#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# The state directory holds command output and saved stash patches, which can
# contain uncommitted changes, so hk creates it private to the user.
@test "hk creates its state directory with owner-only permissions" {
    [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]] && skip "no POSIX permissions"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["fail"] { check = "echo boom; false" }
        }
    }
}
EOF
    echo hi > a.txt

    export HK_STATE_DIR="$BATS_TEST_TMPDIR/state-parent/hk"
    unset HK_OUTPUT_FILE HK_LOG_FILE
    run hk check --all
    assert_failure
    [ -f "$HK_STATE_DIR/output.log" ]

    run ls -ld "$HK_STATE_DIR"
    assert_output --regexp '^drwx------'
    # Directories above the state directory are created as usual.
    run ls -ld "$BATS_TEST_TMPDIR/state-parent"
    refute_output --regexp '^drwx------'
}
