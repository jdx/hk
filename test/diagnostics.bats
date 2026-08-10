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
hooks {
    ["check"] {
        steps {
            ["compiler"] {
                check = "printf 'src/main.c:2:4: warning: first line [W1]\\n  second line\\n' >&2; exit 1"
                output_summary = "combined"
                diagnostic_format = "gcc"
                diagnostic_tool = "cc"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init
}

@test "structured output contains normalized diagnostics and raw output" {
    write_config

    run bash -c "hk --format json check --all 2>/dev/null"
    assert_failure
    run jq -r '.steps[0] | [.diagnostics[0].step, .diagnostics[0].tool, .diagnostics[0].severity, .diagnostics[0].path, .diagnostics[0].range.start.line, .diagnostics[0].rule, (.diagnostics[0].message | contains("second line")), (.output | contains("first line"))] | @tsv' <<<"$output"
    assert_success
    assert_output $'compiler\tcc\twarning\tsrc/main.c\t2\tW1\ttrue\ttrue'
}

@test "SARIF export works independently of the human output format" {
    write_config

    run hk check --all --sarif diagnostics.sarif
    assert_failure
    assert_file_exists diagnostics.sarif
    run jq -r '[.version, .runs[0].tool.driver.name, .runs[0].results[0].ruleId] | @tsv' diagnostics.sarif
    assert_success
    assert_output $'2.1.0\thk\tW1'
}

@test "SARIF write errors are reported even when the hook also fails" {
    write_config

    run hk check --all --sarif hk.pkl/diagnostics.sarif
    assert_failure
    assert_output --partial "failed to emit result after hook also failed"
    assert_file_not_exists hk.pkl/diagnostics.sarif
}

@test "setup failures emit JSON before reporting a SARIF write error" {
    write_config
    mv .git .git.saved

    run bash -c "hk --format json check --all --sarif hk.pkl/diagnostics.sarif 2>machine-errors.log"
    assert_failure
    run jq -e '.schema_version == 1 and .status == "failed"' <<<"$output"
    assert_success
    run grep -F "hook setup also failed" machine-errors.log
    assert_success
    assert_file_not_exists hk.pkl/diagnostics.sarif
}

@test "malformed diagnostic data reports parse warnings without dropping raw output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["eslint"] {
                check = "printf '{bad json' >&2; exit 1"
                output_summary = "combined"
                diagnostic_format = "eslint-json"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "hk --format json check --all 2>/dev/null"
    assert_failure
    run jq -r '[(.steps[0].diagnostics | length), (.steps[0].parse_warnings | length), (.steps[0].output | contains("bad json"))] | @tsv' <<<"$output"
    assert_success
    assert_output $'0\t1\ttrue'
}

@test "steps without captured output do not report parse warnings" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["quiet"] {
                check = "true"
                diagnostic_format = "eslint-json"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "hk --format json check --all 2>/dev/null"
    assert_success
    run jq -e '(.steps[0].parse_warnings // []) == [] and .steps[0].diagnostics == []' <<<"$output"
    assert_success
}

@test "early no-op runs still write an empty SARIF report" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps {} }
}
EOF
    git add .
    git commit -m init

    run hk check --all --sarif diagnostics.sarif
    assert_success
    assert_file_exists diagnostics.sarif
    run jq -e '.version == "2.1.0" and .runs[0].results == []' diagnostics.sarif
    assert_success
}

@test "safe preflight failures still write an empty SARIF report" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["unsafe"] {
                check = new CommandSpec {
                    command = "true"
                    effect = "destructive"
                }
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run hk check --all --safe --sarif diagnostics.sarif
    assert_failure
    assert_file_exists diagnostics.sarif
    run jq -e '.version == "2.1.0" and .runs[0].results == []' diagnostics.sarif
    assert_success
}

@test "structured output preserves check-first diagnostics after a successful fix" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["compiler"] {
                check_diff = "printf 'src/main.c:2:4: warning: fixed issue [W1]\\n' >&2; exit 1"
                fix = "true"
                output_summary = "combined"
                diagnostic_format = "gcc"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "HK_CHECK_FIRST=1 hk --format json check --fix --all 2>/dev/null"
    assert_success
    run jq -r '.steps[0] | [.status, .diagnostics[0].rule, (.output | contains("fixed issue"))] | @tsv' <<<"$output"
    assert_success
    assert_output $'passed\tW1\ttrue'
}

@test "human output preserves check-first diagnostics when the fixer is cancelled" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["compiler"] {
                check = "printf 'src/main.c:2:4: warning: cancelled fix [W1]\\n' >&2; touch ready; exit 1"
                fix = "sleep 5"
                output_summary = "combined"
                diagnostic_format = "gcc"
            }
            ["stop"] {
                check = "while [ ! -f ready ]; do sleep 0.01; done; exit 1"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run env HK_CHECK_FIRST=1 HK_FAIL_FAST=1 hk check --fix --all
    assert_failure
    assert_output --partial "cancelled fix"
}

@test "check-first JSON remains parseable after fixer output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["eslint"] {
                check_diff = "printf '%b' '[{\\042filePath\\042:\\042input.js\\042,\\042messages\\042:[{\\042ruleId\\042:\\042demo\\042,\\042severity\\042:2,\\042message\\042:\\042fix me\\042,\\042line\\042:1,\\042column\\042:1}]}]'; exit 1"
                fix = "echo fixer chatter"
                output_summary = "combined"
                diagnostic_format = "eslint-json"
            }
        }
    }
}
EOF
    touch input.js
    git add .
    git commit -m init

    run bash -c "HK_CHECK_FIRST=1 hk --format json check --fix --all 2>/dev/null"
    assert_success
    run jq -r '.steps[0] | [(.diagnostics | length), .diagnostics[0].rule, (.output | contains("fixer chatter"))] | @tsv' <<<"$output"
    assert_success
    assert_output $'1\tdemo\ttrue'
}

@test "hidden check-first diagnostics remain available to structured output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["compiler"] {
                check_diff = "printf 'input.c:1:1: warning: hidden issue [W1]\\n' >&2; exit 1"
                fix = "true"
                output_summary = "hide"
                diagnostic_format = "gcc"
            }
        }
    }
}
EOF
    touch input.c
    git add .
    git commit -m init

    run bash -c "HK_CHECK_FIRST=1 hk --format json check --fix --all 2>/dev/null"
    assert_success
    run jq -r '.steps[0] | [(.diagnostics | length), .diagnostics[0].rule, (.output | contains("hidden issue"))] | @tsv' <<<"$output"
    assert_success
    assert_output $'1\tW1\ttrue'
}

@test "cancelled focused checks retain their listing diagnostics" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["focused"] {
                check_first = true
                check_failed_files = true
                check_list_files = "printf 'input.txt\\n'; printf 'listing diagnostic\\n' >&2; touch ready; exit 1"
                check = "sleep 5"
                output_summary = "combined"
            }
            ["stop"] {
                check = "while [ ! -f ready ]; do sleep 0.01; done; exit 1"
            }
        }
    }
}
EOF
    touch input.txt
    git add .
    git commit -m init

    run bash -c "HK_FAIL_FAST=1 hk --format json check --all 2>/dev/null"
    assert_failure
    run jq -r '.steps[] | select(.name == "focused") | [.status, (.output | contains("listing diagnostic"))] | @tsv' <<<"$output"
    assert_success
    assert_output $'cancelled\ttrue'
}
