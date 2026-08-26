#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk install creates git hooks" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    hk install --legacy
    assert_file_exists ".git/hooks/pre-commit"

    run cat ".git/hooks/pre-commit"
    assert_success
    assert_output --partial 'run pre-commit --from-hook "$@"'
    refute_output --partial '--staged'
}
