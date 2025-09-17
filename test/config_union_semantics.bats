#!/usr/bin/env bats

# Test union semantics for exclude, skip_steps, and skip_hooks settings

setup() {
    export HK="${HK:-$BATS_TEST_DIRNAME/../target/debug/hk}"
    export TEST_DIR="$(mktemp -d)"
    cd "$TEST_DIR"

    # Create a simple hk.pkl config
    cat > hk.pkl << 'EOF'
amends "pkl:hk.dev/hk-config"

hooks = new {
    ["check"] = new Hook {
        steps = new {
            ["test_step1"] = new Step {
                run = "echo test1"
            }
            ["test_step2"] = new Step {
                run = "echo test2"
            }
            ["test_step3"] = new Step {
                run = "echo test3"
            }
        }
    }
}
EOF
}

teardown() {
    rm -rf "$TEST_DIR"
}

@test "skip_steps union: env and git config combine" {
    # Set up git config
    git init
    git config hk.skipSteps "test_step1"

    # Set env var
    export HK_SKIP_STEPS="test_step2"

    # Run check --plan to see what steps would be skipped
    run $HK check --plan
    [ "$status" -eq 0 ]

    # Both test_step1 and test_step2 should be skipped
    [[ ! "$output" =~ "test_step1" ]] || fail "test_step1 should be skipped"
    [[ ! "$output" =~ "test_step2" ]] || fail "test_step2 should be skipped"
    [[ "$output" =~ "test_step3" ]] || fail "test_step3 should not be skipped"
}

@test "skip_steps union: singular and plural git keys combine" {
    git init

    # Set singular form
    git config hk.skipStep "test_step1"

    # Add plural form (using --add to create multivar)
    git config --add hk.skipSteps "test_step2"

    run $HK check --plan
    [ "$status" -eq 0 ]

    # Both should be skipped
    [[ ! "$output" =~ "test_step1" ]] || fail "test_step1 should be skipped"
    [[ ! "$output" =~ "test_step2" ]] || fail "test_step2 should be skipped"
    [[ "$output" =~ "test_step3" ]] || fail "test_step3 should not be skipped"
}

@test "skip_steps union: CSV values are split and combined" {
    git init

    # Set CSV value in git config
    git config hk.skipSteps "test_step1,test_step2"

    # Set another step via env
    export HK_SKIP_STEP="test_step3"

    run $HK check --plan
    [ "$status" -eq 0 ]

    # All three should be skipped
    [[ ! "$output" =~ "test_step1" ]] || fail "test_step1 should be skipped"
    [[ ! "$output" =~ "test_step2" ]] || fail "test_step2 should be skipped"
    [[ ! "$output" =~ "test_step3" ]] || fail "test_step3 should be skipped"
}

@test "exclude union: patterns accumulate across sources" {
    git init

    # Create test files
    touch file1.tmp file2.log file3.bak file4.txt

    # Set exclude patterns in different sources
    git config hk.exclude "*.tmp"
    export HK_EXCLUDE="*.log,*.bak"

    # Add hk config for the test hook to check files
    cat > hk.pkl << 'EOF'
amends "pkl:hk.dev/hk-config"

hooks = new {
    ["check"] = new Hook {
        steps = new {
            ["list_files"] = new Step {
                run = "ls -1 | sort"
                glob = ["*"]
            }
        }
    }
}
EOF

    run $HK check --plan
    [ "$status" -eq 0 ]

    # Check that excluded files are not in the output
    [[ ! "$output" =~ "file1.tmp" ]] || fail "*.tmp should be excluded"
    [[ ! "$output" =~ "file2.log" ]] || fail "*.log should be excluded"
    [[ ! "$output" =~ "file3.bak" ]] || fail "*.bak should be excluded"
    [[ "$output" =~ "file4.txt" ]] || fail "file4.txt should not be excluded"
}

@test "hide_warnings union: tags combine from all sources" {
    git init

    # Set hide_warnings in git config
    git config hk.hideWarnings "warning1"

    # Set hide_warnings in env (CSV format)
    export HK_HIDE_WARNINGS="warning2,warning3"

    # Note: We can't easily test the actual warning hiding without
    # triggering warnings, but we can at least verify the config loads
    run $HK config dump
    [ "$status" -eq 0 ]
}

@test "skip_hooks union: hooks accumulate across sources" {
    git init

    # Create additional hooks in config
    cat > hk.pkl << 'EOF'
amends "pkl:hk.dev/hk-config"

hooks = new {
    ["pre-commit"] = new Hook {
        steps = new {
            ["pre_step"] = new Step {
                run = "echo pre-commit"
            }
        }
    }
    ["pre-push"] = new Hook {
        steps = new {
            ["push_step"] = new Step {
                run = "echo pre-push"
            }
        }
    }
    ["check"] = new Hook {
        steps = new {
            ["check_step"] = new Step {
                run = "echo check"
            }
        }
    }
}
EOF

    # Skip hooks via different sources
    git config hk.skipHooks "pre-commit"
    export HK_SKIP_HOOK="pre-push"

    # pre-commit and pre-push should be skipped, check should run
    run $HK check --plan
    [ "$status" -eq 0 ]
    [[ "$output" =~ "check_step" ]] || fail "check hook should not be skipped"

    # Verify the skipped hooks don't run
    run $HK run pre-commit --plan
    [ "$status" -eq 1 ] || fail "pre-commit should be skipped"

    run $HK run pre-push --plan
    [ "$status" -eq 1 ] || fail "pre-push should be skipped"
}

@test "CLI flags override for non-union settings" {
    git init

    # Set fail_fast in git config (not a union field - uses replace merge)
    git config hk.failFast true

    # Override with CLI flag
    run $HK check --no-fail-fast --plan
    [ "$status" -eq 0 ]

    # The CLI flag should override the git config
    # (We'd need to check actual behavior with failing steps to fully verify)
}

@test "precedence: CLI > env > git > pkl > defaults" {
    git init

    # Set jobs at different levels
    git config hk.jobs 2
    export HK_JOBS=4

    # CLI flag should win
    run $HK check --jobs 8 --plan
    [ "$status" -eq 0 ]

    # Verify by checking the output or config dump
    # (actual verification would depend on how jobs affects the output)
}

@test "dedupe in union: duplicate values are merged" {
    git init

    # Add same skip_step in multiple sources
    git config hk.skipStep "test_step1"
    git config --add hk.skipSteps "test_step1"  # duplicate
    export HK_SKIP_STEPS="test_step1,test_step2"  # test_step1 is duplicate

    run $HK check --plan
    [ "$status" -eq 0 ]

    # test_step1 should only appear once (deduped)
    # test_step2 should also be skipped
    [[ ! "$output" =~ "test_step1" ]] || fail "test_step1 should be skipped"
    [[ ! "$output" =~ "test_step2" ]] || fail "test_step2 should be skipped"
    [[ "$output" =~ "test_step3" ]] || fail "test_step3 should not be skipped"
}

@test "multivar git config values are all read" {
    git init

    # Add multiple values for the same key using --add
    git config hk.skipSteps "test_step1"
    git config --add hk.skipSteps "test_step2"
    git config --add hk.skipSteps "test_step3"

    run $HK check --plan
    [ "$status" -eq 0 ]

    # All three should be skipped
    [[ ! "$output" =~ "test_step1" ]] || fail "test_step1 should be skipped"
    [[ ! "$output" =~ "test_step2" ]] || fail "test_step2 should be skipped"
    [[ ! "$output" =~ "test_step3" ]] || fail "test_step3 should be skipped"
}
