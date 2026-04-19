#!/usr/bin/env bats

load 'test_helper/common_setup'

setup() {
    _common_setup
}
teardown() {
    _common_teardown
}

@test "hk --plan shows basic plan" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js", "*.ts")
                check = "prettier --check {{files}}"
            }
            ["cargo-fmt"] {
                glob = List("*.rs")
                check = "cargo fmt --check"
            }
        }
    }
}
EOF
    touch file.js file.rs
    git add .
    run hk check --plan
    assert_success
    assert_output --partial "Plan: check"
    assert_output --partial "prettier"
    assert_output --partial "cargo-fmt"
}

@test "hk --plan with no matching files marks steps skipped" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js")
                check = "prettier --check {{files}}"
            }
        }
    }
}
EOF
    touch file.txt
    git add .
    run hk check --plan
    assert_success
    assert_output --partial "Plan: check"
    assert_output --partial "prettier"
    assert_output --partial "no files matched"
}

@test "hk --plan --json outputs valid JSON" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js")
                check = "prettier --check {{files}}"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.hook == "check"' >/dev/null
    echo "$output" | jq -e '.runType == "check"' >/dev/null
    echo "$output" | jq -e '.steps | length > 0' >/dev/null
    echo "$output" | jq -e '.generatedAt' >/dev/null
}

@test "hk --plan --json includes profiles when set" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["s"] {
                glob = List("*.js")
                check = "echo"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk --profile fast check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.profiles | contains(["fast"])' >/dev/null
}

@test "hk --plan with profile-gated step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["fast-check"] {
                glob = List("*.js")
                check = "echo fast"
                profiles = List("fast")
            }
            ["default-check"] {
                glob = List("*.js")
                check = "echo default"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan
    assert_success
    assert_output --partial "fast-check"
    assert_output --partial "default-check"
    assert_output --partial "profile"

    run bash -c "hk --profile fast check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "fast-check") | .status == "included"' >/dev/null
}

@test "hk --plan with condition=false" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["cond-false"] {
                glob = List("*.js")
                check = "echo nope"
                condition = "false"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "cond-false") | .status == "skipped"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "cond-false") | .reasons[] | select(.kind == "condition_false")' >/dev/null
}

@test "hk --plan with condition=true" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["cond-true"] {
                glob = List("*.js")
                check = "echo yes"
                condition = "true"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "cond-true") | .status == "included"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "cond-true") | .reasons[] | select(.kind == "condition_true")' >/dev/null
}

@test "hk --plan shows step dependencies in JSON" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step-a"] {
                glob = List("*.js")
                check = "echo a"
            }
            ["step-b"] {
                glob = List("*.js")
                check = "echo b"
                depends = List("step-a")
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "step-b") | .dependsOn | contains(["step-a"])' >/dev/null
}

@test "hk --plan respects --step flag" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { glob = List("*.js"); check = "echo a" }
            ["b"] { glob = List("*.js"); check = "echo b" }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --step a --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '[.steps[].name] == ["a"]' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "a") | .reasons[] | select(.kind == "cli_include")' >/dev/null
}

@test "hk --plan respects --skip-step flag" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["keep"] { glob = List("*.js"); check = "echo keep" }
            ["drop"] { glob = List("*.js"); check = "echo drop" }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --skip-step drop --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "drop") | .status == "skipped"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "drop") | .reasons[] | select(.kind == "cli_exclude")' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "keep") | .status == "included"' >/dev/null
}

@test "hk --plan does not execute commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["boom"] {
                glob = List("*.js")
                check = "touch executed.marker && exit 1"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan
    assert_success
    refute [ -e executed.marker ]
}

@test "hk --why <step> focuses on one step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { glob = List("*.js"); check = "echo a" }
            ["b"] { glob = List("*.js"); check = "echo b" }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --why a
    assert_success
    assert_output --partial "Plan: check"
    assert_output --partial "a"
    refute_output --partial "✓ b"
    refute_output --partial "○ b"
}

@test "hk --why shows all steps with detailed reasons" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] {
                glob = List("*.js")
                check = "echo a"
                condition = "true"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --why
    assert_success
    assert_output --partial "Plan: check"
    assert_output --partial "a"
    assert_output --partial "condition evaluated to true"
}

# Regression: when both job_condition and profiles are set, build_step_jobs
# defers profile checks to runtime. The plan must still report profile-skip.
@test "hk --plan profile-skipped step with condition is reported skipped" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["gated"] {
                glob = List("*.js")
                check = "echo gated"
                condition = "true"
                profiles = List("slow")
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "gated") | .status == "skipped"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "gated") | .reasons[] | select(.kind == "profile_exclude")' >/dev/null
}

# Regression: runtime treats non-bool expression results (e.g. strings from
# exec()) as truthy. The plan must match that behavior.
@test "hk --plan treats non-bool condition result as truthy" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["stringy"] {
                glob = List("*.js")
                check = "echo ok"
                condition = "'yes'"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "stringy") | .status == "included"' >/dev/null
}

# Regression: step_condition is evaluated in execution.rs before build_step_jobs
# and can skip the step entirely.
@test "hk --plan evaluates step_condition" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["gated"] {
                glob = List("*.js")
                check = "echo gated"
                step_condition = "false"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "gated") | .status == "skipped"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "gated") | .reasons[] | select(.kind == "condition_false")' >/dev/null
}

# --why --json should not raise a clap error about --plan being required.
@test "hk --why --json works without explicit --plan" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { glob = List("*.js"); check = "echo a" }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --why --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.hook == "check"' >/dev/null
}

# Regression: profile-skipped step should still report matched file count.
@test "hk --plan profile-skipped step reports matched fileCount" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["gated"] {
                glob = List("*.js")
                check = "echo gated"
                profiles = List("slow")
            }
        }
    }
}
EOF
    touch a.js b.js c.js
    git add .
    run bash -c "hk check --plan --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '.steps[] | select(.name == "gated") | .status == "skipped"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "gated") | .fileCount == 3' >/dev/null
}

# Regression: a skipped step with a truthy condition should be headlined by
# the decisive skip reason, not "condition evaluated to true".
@test "hk --plan skipped step headline shows decisive skip reason, not truthy condition" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ghost"] {
                glob = List("*.xyz")
                check = "echo ghost"
                condition = "true"
            }
        }
    }
}
EOF
    touch file.txt
    git add .
    run hk check --plan
    assert_success
    # The headline (parenthesized on the step line) must be the skip reason.
    assert_output --partial "○ ghost  (no files matched filters)"
    refute_output --partial "○ ghost  (condition evaluated to true"
}

# Regression: verbose --why on a skipped step whose headline is picked from a
# non-zero index must not duplicate the headline or drop the first reason.
# --json alone (without --plan/--why/--trace) should error rather than
# silently running the hook with no JSON output.
@test "hk check --json without --plan or --why errors" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { glob = List("*.js"); check = "echo a" }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --json
    assert_failure
    assert_output --partial "--json requires --plan or --why"
}

# The global --trace --json combination must still work (json controls
# tracing output format in that mode).
@test "hk --trace --json still emits JSON trace output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { glob = List("*.js"); check = "echo a" }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk --trace --json check 2>/dev/null"
    assert_success
    assert_output --partial '"type":"meta"'
}

# Regression: --why <step> --json should filter JSON output to the focused
# step, mirroring the text renderer. Parallel-group stepIds must also be
# pruned so they never reference steps that have been filtered out.
@test "hk --why <step> --json filters JSON to focused step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { glob = List("*.js"); check = "echo a" }
            ["b"] { glob = List("*.js"); check = "echo b" }
            ["c"] { glob = List("*.js"); check = "echo c" }
        }
    }
}
EOF
    touch file.js
    git add .
    run bash -c "hk check --why a --json 2>/dev/null"
    assert_success
    echo "$output" | jq -e '[.steps[].name] == ["a"]' >/dev/null
    # When the group survives (it contains the focused step), its stepIds
    # must only reference steps present in the filtered steps array.
    echo "$output" | jq -e '
        .groups == [] or
        (.groups | all(.stepIds | all(. == "a")))
    ' >/dev/null
}

@test "hk --why skipped step shows each reason exactly once" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ghost"] {
                glob = List("*.xyz")
                check = "echo ghost"
                step_condition = "true"
            }
        }
    }
}
EOF
    touch file.txt
    git add .
    run hk check --why ghost
    assert_success
    # Headline line with the decisive skip reason appears exactly once.
    matched=$(echo "$output" | grep -c "no files matched filters" || true)
    [ "$matched" = "1" ]
    # And the truthy-condition reason (pushed first in the reasons vec) is
    # still surfaced in the verbose detail list.
    assert_output --partial "step_condition evaluated to true"
}
