#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "batch=false should skip when no files match" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
display_skip_reasons = List("no-files-to-process")
hooks {
    ["fix"] {
        steps {
            ["prettier"] {
                batch = false
                glob = "*.nonexistent"
                fix = "prettier --write {{ files }}"
            }
        }
    }
}
EOF
    run hk fix --all
    assert_success
    assert_output --partial "prettier – skipped: no files to process"
    refute_output --partial "[error] No parser and no file path given"
}

@test "batch=false with exclude should skip when all files excluded" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
display_skip_reasons = List("no-files-to-process")
hooks {
    ["fix"] {
        steps {
            ["custom"] {
                batch = false
                exclude = "*.pkl"
                fix = "echo 'should not run' {{ files }}"
            }
        }
    }
}
EOF
    # hk.pkl will be in git status but excluded
    run hk fix
    assert_success
    assert_output --partial "custom – skipped: no files to process"
    refute_output --partial "should not run"
}

