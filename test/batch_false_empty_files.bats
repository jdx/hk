#!/usr/bin/env mise run test:bats

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

@test "batch=false should handle files deleted between collection and execution" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
display_skip_reasons = List("no-files-to-process")
hooks {
    ["fix"] {
        steps {
            ["deleter"] {
                exclusive = true
                glob = "*.temp.mjs"
                fix = "rm -f {{ files }}"
            }
            ["prettier"] {
                batch = false
                glob = "*.temp.mjs"
                fix = "prettier --write {{ files }}"
            }
        }
    }
}
EOF
    # Create a temp file that matches the glob
    echo "export const foo = 1;" > test.temp.mjs
    git add test.temp.mjs hk.pkl
    git commit -m "add temp file"

    # The "deleter" step runs first (exclusive=true) and removes the file
    # Then "prettier" step tries to run but file is gone
    # On origin/main this would fail with prettier error
    # With the fix, it should skip gracefully
    run hk fix --all
    assert_success
    assert_output --partial "prettier – skipped: no files to process"
    refute_output --partial "[error] No files matching the pattern"
}


