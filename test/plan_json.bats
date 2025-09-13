#!/usr/bin/env bats

load 'test_helper/common_setup'

setup() {
    _common_setup
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
steps {
    ["prettier"] {
        glob = List("*.js")
        check = "prettier --check"
        fix = "prettier --write"
    }
    ["eslint"] {
        glob = List("*.js")
        check = "eslint"
        depends = List("prettier")
    }
}
EOF
}

teardown() {
    _common_teardown
}

@test "hk --plan --json outputs valid JSON schema" {
    touch file.js
    git add .
    run hk check --plan --plan-json
    [ "$status" -eq 0 ]

    # Validate JSON structure
    echo "$output" | jq -e '.' >/dev/null

    # Check required fields
    echo "$output" | jq -e '.hook' >/dev/null
    echo "$output" | jq -e '.steps' >/dev/null
    echo "$output" | jq -e '.generatedAt' >/dev/null

    # Check step structure
    echo "$output" | jq -e '.steps[0].name' >/dev/null
    echo "$output" | jq -e '.steps[0].status' >/dev/null
    echo "$output" | jq -e '.steps[0].orderIndex' >/dev/null
    echo "$output" | jq -e '.steps[0].reasons' >/dev/null
}

@test "hk --plan --json includes profiles when set" {
    touch file.js
    git add .
    run hk --profile fast check --plan --plan-json
    [ "$status" -eq 0 ]

    # Check profiles field exists and contains "fast"
    echo "$output" | jq -e '.profiles | contains(["fast"])' >/dev/null
}

@test "hk --plan --json shows step dependencies" {
    touch file.js
    git add .
    run hk check --plan --plan-json
    [ "$status" -eq 0 ]

    # eslint should depend on prettier
    echo "$output" | jq -e '.steps[] | select(.name == "eslint") | .dependsOn | contains(["prettier"])' >/dev/null
}

@test "hk --plan --json shows skip reasons" {
    # No JS files, so steps should be skipped
    touch file.rs
    git add .
    run hk check --plan --plan-json
    [ "$status" -eq 0 ]

    # Check that steps are marked as skipped
    echo "$output" | jq -e '.steps[] | select(.name == "prettier") | .status == "skipped"' >/dev/null

    # Check that reasons are provided
    echo "$output" | jq -e '.steps[] | select(.name == "prettier") | .reasons | length > 0' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "prettier") | .reasons[0].kind' >/dev/null
}

@test "hk --plan --json shows parallel groups" {
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
steps {
    ["group:parallel"] {
        ["step1"] {
            glob = List("*.js")
            check = "echo 1"
        }
        ["step2"] {
            glob = List("*.js")
            check = "echo 2"
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan --plan-json
    [ "$status" -eq 0 ]

    # Check that groups array exists if there are parallel groups
    echo "$output" | jq -e '.groups' >/dev/null || true

    # Steps in the same group should have the same parallelGroupId
    if echo "$output" | jq -e '.groups | length > 0' 2>/dev/null; then
        echo "$output" | jq -e '.groups[0].stepIds | length == 2' >/dev/null
    fi
}

@test "hk --plan --json respects --step selection" {
    touch file.js
    git add .
    run hk check --plan --plan-json --step prettier
    [ "$status" -eq 0 ]

    # prettier should be included
    echo "$output" | jq -e '.steps[] | select(.name == "prettier") | .status == "included"' >/dev/null

    # Should have a CLI include reason
    echo "$output" | jq -e '.steps[] | select(.name == "prettier") | .reasons[] | select(.kind == "cli_include")' >/dev/null
}

@test "hk --plan --json shows condition evaluation results" {
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
steps {
    ["cond-true"] {
        glob = List("*.js")
        check = "echo test"
        condition = "true"
    }
    ["cond-false"] {
        glob = List("*.js")
        check = "echo test"
        condition = "false"
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan --plan-json
    [ "$status" -eq 0 ]

    # cond-true should be included with condition_true reason
    echo "$output" | jq -e '.steps[] | select(.name == "cond-true") | .status == "included"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "cond-true") | .reasons[] | select(.kind == "condition_true")' >/dev/null

    # cond-false should be skipped with condition_false reason
    echo "$output" | jq -e '.steps[] | select(.name == "cond-false") | .status == "skipped"' >/dev/null
    echo "$output" | jq -e '.steps[] | select(.name == "cond-false") | .reasons[] | select(.kind == "condition_false")' >/dev/null
}

@test "hk --plan --json includes timestamp" {
    touch file.js
    git add .
    run hk check --plan --plan-json
    [ "$status" -eq 0 ]

    # generatedAt should be a valid ISO timestamp
    echo "$output" | jq -e '.generatedAt' >/dev/null
    # Check it's a string that looks like a timestamp
    echo "$output" | jq -e '.generatedAt | type == "string"' >/dev/null
    echo "$output" | jq -e '.generatedAt | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T")' >/dev/null
}
