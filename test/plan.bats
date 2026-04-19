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
