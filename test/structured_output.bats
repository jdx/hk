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

@test "--format is accepted after check and run subcommands" {
    write_config

    run bash -c "hk check --format json --all 2>/dev/null"
    assert_failure
    run jq -e '.kind == "run_result"' <<<"$output"
    assert_success

    run bash -c "hk run check --format json --all 2>/dev/null"
    assert_failure
    run jq -e '.kind == "run_result"' <<<"$output"
    assert_success
}

@test "--format jsonl emits ordered lifecycle events" {
    write_config

    run bash -c "hk --format jsonl check --all 2>machine-errors.log"
    assert_failure
    jsonl="$output"
    run jq -s -r 'map(.event) | join(",")' <<<"$jsonl"
    assert_success
    assert_output "run_started,run_planned,step_started,step_started,step_completed,step_completed,run_completed"
    run jq -s -e '.[].sequence' <<<"$jsonl"
    assert_success
    run jq -s -e 'last.data.status == "failed" and (last.data.failure | length > 0)' <<<"$jsonl"
    assert_success
}

@test "human output remains the default" {
    write_config

    run hk check --all
    assert_failure
    run jq -e . <<<"$output"
    assert_failure
}

@test "global execution format coexists with config dump format" {
    run hk config dump --format json
    assert_success
    run jq -e . <<<"$output"
    assert_success

    run hk config dump --format toml --help
    assert_success

    run hk --format json config dump --format json
    assert_success
    run jq -e . <<<"$output"
    assert_success
}

@test "fail-fast marks unrun steps cancelled" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
    ["check"] {
        steps {
            ["fail"] {
                check = "exit 1"
            }
            ["unrun"] {
                depends = List("fail")
                check = "true"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_failure
    run jq -r '.steps[] | select(.name == "unrun") | .status' <<<"$output"
    assert_success
    assert_output "cancelled"
}

@test "fail-fast marks an in-flight timed step cancelled" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
    ["check"] {
        steps {
            ["parallel"] = new Group {
                steps {
                    ["slow"] { check = "touch slow-started; sleep 2" }
                    ["fail"] { check = "sleep 0.5; exit 1" }
                }
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_failure
    assert_file_exists slow-started
    run jq -r '.steps[] | select(.name == "slow") | .status' <<<"$output"
    assert_success
    assert_output "cancelled"
}

@test "installed-hook no-ops emit a structured reason before config loading" {
    rm -f hk.pkl

    run bash -c "hk --format json run pre-commit --from-hook 2>machine-errors.log"
    assert_success
    run jq -r '[.status, (.steps | length), .reason] | @tsv' <<<"$output"
    assert_success
    assert_output $'passed\t0\tno project configuration found for installed hook'
}

@test "no-op runs still emit a complete result" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {}
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_success
    run jq -r '[.status, (.steps | length), .reason] | @tsv' <<<"$output"
    assert_success
    assert_output $'passed\t0\tno configured steps'
}

@test "early no-file exit preserves each step's skip reason" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["files"] {
                check = "true"
                glob = List("*.rs")
            }
            ["profile"] {
                check = "true"
                profiles = List("slow")
            }
        }
    }
}
EOF
    git add .
    git commit -m init

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_success
    run jq -r '.steps[] | [.name, .skip_reason] | @tsv' <<<"$output"
    assert_success
    assert_output --partial $'files\tskipped: no files to process'
    assert_output --partial $'profile\tskipped: profile not enabled (slow)'
}

@test "conditions take precedence over an early no-files reason" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["conditional"] {
                check = "true"
                glob = List("*.rs")
                step_condition = "false"
            }
        }
    }
}
EOF
    git add .
    git commit -m init

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_success
    run jq -r '.steps[0] | [.status, .skip_reason] | @tsv' <<<"$output"
    assert_success
    assert_output $'skipped\tskipped: condition is false'
}

@test "empty batch jobs remain a no-op with a skip reason" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["batch"] {
                check = "true"
                batch = true
            }
        }
    }
}
EOF
    git add .
    git commit -m init
    : >files.list

    run bash -c "hk --format json check --files0-from files.list 2>/dev/null"
    assert_success
    run jq -r '.steps[0] | [.status, .skip_reason] | @tsv' <<<"$output"
    assert_success
    assert_output $'skipped\tskipped: no files to process'
}

@test "JSON tracing cannot contaminate structured stdout" {
    write_config

    run bash -c "HK_TRACE=json hk --format json check --all 2>machine-errors.log"
    assert_failure
    run jq -e '.kind == "run_result"' <<<"$output"
    assert_success
    run bash -c "grep '\"type\":\"meta\"' machine-errors.log | jq -e '.type == \"meta\"'"
    assert_success
}

@test "setup failures still emit a versioned failed result" {
    write_config
    mv .git .git.saved

    run bash -c "hk --format json check --all 2>machine-errors.log"
    assert_failure
    run jq -r '[.schema_version, .status, (.failure | length > 0)] | @tsv' <<<"$output"
    assert_success
    assert_output $'1\tfailed\ttrue'
}
