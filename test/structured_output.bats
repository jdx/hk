#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
    ["check"] {
        steps {
            ["pass"] {
                check = "echo passed"
                output_summary = "combined"
            }
            ["fail"] {
                check = "echo diagnostic >&2; exit 1"
                output_summary = "combined"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init
}

@test "--format json emits one versioned run result" {
    write_config

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_failure
    json="$output"
    run jq -r '.schema_version, .kind, .hook, .status, (.steps | length)' <<<"$json"
    assert_success
    assert_output $'1\nrun_result\ncheck\nfailed\n2'
    run jq -r '.steps[] | select(.name == "fail") | [.status, (.output | contains("diagnostic"))] | @tsv' <<<"$json"
    assert_output $'failed\ttrue'
}

@test "--format jsonl emits ordered lifecycle events" {
    write_config

    run bash -c "hk --format jsonl check --all 2>machine-errors.log"
    assert_failure
    jsonl="$output"
    run jq -s -r 'map(.event) | join(",")' <<<"$jsonl"
    assert_success
    assert_output "run_started,step_completed,step_completed,run_completed"
    run jq -s -e '.[].sequence' <<<"$jsonl"
    assert_success
}

@test "human output remains the default" {
    write_config

    run hk check --all
    assert_failure
    run jq -e . <<<"$output"
    assert_failure
}
