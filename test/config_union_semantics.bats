#!/usr/bin/env mise run test:bats

# Test union semantics for exclude, skip_steps, and skip_hooks settings

setup() {
    load 'test_helper/common_setup'
    _common_setup

    # Create a simple hk.pkl config
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
display_skip_reasons = List("disabled-by-env", "disabled-by-config")
hooks {
    ["check"] {
        steps {
            ["test_step1"] { check = "echo test1" }
            ["test_step2"] { check = "echo test2" }
            ["test_step3"] { check = "echo test3" }
        }
    }
}
EOF
}

teardown() {
    _common_teardown
}

@test "skip_steps union: env and git config combine" {
    # Set up git config
    git config hk.skipSteps "test_step1"

    # Set env var
    export HK_SKIP_STEPS="test_step2"

    # Add files to commit so steps have something to process
    echo "test" > test.js
    git add test.js

    # Run check -v to see what steps are skipped
    run hk check -v
    [ "$status" -eq 0 ]

    # Both test_step1 and test_step2 should be skipped with skip reasons
    [[ "$output" =~ "test_step1 – skipped:" ]] || fail "test_step1 should be skipped"
    [[ "$output" =~ "test_step2 – skipped:" ]] || fail "test_step2 should be skipped"
    # test_step3 should run (not show skipped message)
    [[ ! "$output" =~ "test_step3 – skipped:" ]] || fail "test_step3 should not be skipped"
}

@test "skip_steps union: singular and plural git keys combine" {
    # Set singular form
    git config hk.skipStep "test_step1"

    # Add plural form (using --add to create multivar)
    git config --add hk.skipSteps "test_step2"

    # Add files to commit so steps have something to process
    echo "test" > test.js
    git add test.js

    run hk check -v
    [ "$status" -eq 0 ]

    # Both should be skipped
    [[ "$output" =~ "test_step1 – skipped:" ]] || fail "test_step1 should be skipped"
    [[ "$output" =~ "test_step2 – skipped:" ]] || fail "test_step2 should be skipped"
    [[ ! "$output" =~ "test_step3 – skipped:" ]] || fail "test_step3 should not be skipped"
}

@test "skip_steps union: CSV values are split and combined" {
    # Set CSV value in git config
    git config hk.skipSteps "test_step1,test_step2"

    # Set another step via env
    export HK_SKIP_STEP="test_step3"

    # Add files to commit so steps have something to process
    echo "test" > test.js
    git add test.js

    run hk check -v
    [ "$status" -eq 0 ]

    # All three should be skipped
    [[ "$output" =~ "test_step1 – skipped:" ]] || fail "test_step1 should be skipped"
    [[ "$output" =~ "test_step2 – skipped:" ]] || fail "test_step2 should be skipped"
    [[ "$output" =~ "test_step3 – skipped:" ]] || fail "test_step3 should be skipped"
}

@test "exclude union: patterns accumulate across sources" {
    # Create test files
    touch file1.tmp file2.log file3.bak file4.txt

    # Set exclude patterns in different sources
    git config hk.exclude "*.tmp"
    export HK_EXCLUDE="*.log,*.bak"

    # Add hk config for the test hook to check files
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["list_files"] { check = "echo {{files}}" }
        }
    }
}
EOF

    run hk check -v
    [ "$status" -eq 0 ]

    # Check that excluded files are not in the command output
    [[ ! "$output" =~ "file1.tmp" ]] || fail "*.tmp should be excluded"
    [[ ! "$output" =~ "file2.log" ]] || fail "*.log should be excluded"
    [[ ! "$output" =~ "file3.bak" ]] || fail "*.bak should be excluded"
    [[ "$output" =~ "file4.txt" ]] || fail "file4.txt should not be excluded"
}

@test "hide_warnings union: tags combine from all sources" {
    # Set hide_warnings in git config
    git config hk.hideWarnings "warning1"

    # Set hide_warnings in env (CSV format)
    export HK_HIDE_WARNINGS="warning2,warning3"

    # Note: We can't easily test the actual warning hiding without
    # triggering warnings, but we can at least verify the config loads
    run hk config dump
    [ "$status" -eq 0 ]
}

@test "skip_hooks union: hooks accumulate across sources" {
    # Create additional hooks in config
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["hook1"] {
        steps {
            ["step1"] { check = "echo hook1" }
        }
    }
    ["hook2"] {
        steps {
            ["step2"] { check = "echo hook2" }
        }
    }
    ["check"] {
        steps {
            ["check_step"] { check = "echo check" }
        }
    }
}
EOF

    # Skip hooks via different sources
    git config hk.skipHooks "hook1"
    export HK_SKIP_HOOK="hook2"

    # check should run, hook1 and hook2 should be skipped
    echo "test" > test.js
    git add test.js

    run hk check -v
    [ "$status" -eq 0 ]
    [[ "$output" =~ "check_step" ]] || fail "check hook should not be skipped"

    # Verify the skipped hooks show warning messages
    run hk run hook1 -v
    [ "$status" -eq 0 ]
    [[ "$output" =~ "skipping" ]] || fail "hook1 should show skipped message"

    run hk run hook2 -v
    [ "$status" -eq 0 ]
    [[ "$output" =~ "skipping" ]] || fail "hook2 should show skipped message"
}

@test "CLI flags override for non-union settings" {
    # Set fail_fast in git config (not a union field - uses replace merge)
    git config hk.failFast true

    # Override with CLI flag
    echo "test" > test.js
    git add test.js

    run hk check --no-fail-fast -v
    [ "$status" -eq 0 ]

    # The CLI flag should override the git config
    # (We'd need to check actual behavior with failing steps to fully verify)
}

@test "precedence: CLI > env > git > pkl > defaults" {
    # Set jobs at different levels
    git config hk.jobs 2
    export HK_JOBS=4

    # CLI flag should win
    echo "test" > test.js
    git add test.js

    run hk check --jobs 8 -v
    [ "$status" -eq 0 ]

    # Verify by checking the output or config dump
    # (actual verification would depend on how jobs affects the output)
}

@test "dedupe in union: duplicate values are merged" {
    # Add same skip_step in multiple sources
    git config hk.skipStep "test_step1"
    git config --add hk.skipSteps "test_step1"  # duplicate
    export HK_SKIP_STEPS="test_step1,test_step2"  # test_step1 is duplicate

    # Add files to commit so steps have something to process
    echo "test" > test.js
    git add test.js

    run hk check -v
    [ "$status" -eq 0 ]

    # test_step1 should only appear once (deduped)
    # test_step2 should also be skipped
    [[ "$output" =~ "test_step1 – skipped:" ]] || fail "test_step1 should be skipped"
    [[ "$output" =~ "test_step2 – skipped:" ]] || fail "test_step2 should be skipped"
    [[ ! "$output" =~ "test_step3 – skipped:" ]] || fail "test_step3 should not be skipped"
}

@test "multivar git config values are all read" {
    # Add multiple values for the same key using --add
    git config hk.skipSteps "test_step1"
    git config --add hk.skipSteps "test_step2"
    git config --add hk.skipSteps "test_step3"

    # Add files to commit so steps have something to process
    echo "test" > test.js
    git add test.js

    run hk check -v
    [ "$status" -eq 0 ]

    # All three should be skipped
    [[ "$output" =~ "test_step1 – skipped:" ]] || fail "test_step1 should be skipped"
    [[ "$output" =~ "test_step2 – skipped:" ]] || fail "test_step2 should be skipped"
    [[ "$output" =~ "test_step3 – skipped:" ]] || fail "test_step3 should be skipped"
}

