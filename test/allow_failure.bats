#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "allow_failure reports command failure without failing hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
    ["check"] {
        steps {
            ["allowed"] {
                check = "echo ALLOWED_FAILURE >&2; exit 1"
                allow_failure = true
                exclusive = true
            }
            ["required"] {
                check = "echo REQUIRED_RAN"
            }
        }
    }
}
EOF

    run hk check --all
    assert_success
    assert_output --partial "ALLOWED_FAILURE"
    assert_output --partial "REQUIRED_RAN"
}

@test "allow_failure defaults to false" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["blocking"] {
                check = "echo BLOCKING_FAILURE >&2; exit 1"
            }
        }
    }
}
EOF

    run hk check --all
    assert_failure
    assert_output --partial "BLOCKING_FAILURE"
}

@test "allow_failure can be controlled by environment" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["conditional"] {
                check = "echo CONDITIONAL_FAILURE >&2; exit 1"
                allow_failure = "env('KNOWN_BROKEN') == 'true'"
            }
        }
    }
}
EOF

    run hk check --all
    assert_failure

    export KNOWN_BROKEN=true
    run hk check --all
    assert_success
    assert_output --partial "CONDITIONAL_FAILURE"
}

@test "allow_failure does not hide hk execution errors" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["broken-template"] {
                check = "echo {{ missing_template_value }}"
                allow_failure = true
            }
        }
    }
}
EOF

    run hk check --all
    assert_failure
    assert_output --partial "failed to render command template"
}

@test "structured output identifies an allowed failing step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["allowed"] {
                check = "echo JSON_FAILURE >&2; exit 1"
                allow_failure = true
            }
        }
    }
}
EOF

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_success
    json="$output"
    run jq -e '.status == "passed" and .steps[0].status == "failed" and .steps[0].failure_allowed == true' <<<"$json"
    assert_success
}
