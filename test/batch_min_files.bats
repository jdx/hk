#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Each batch appends one line with its file count.
batch_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["count"] {
                glob = "*.txt"
                batch = true
                $1
                check = "echo {{ files }} | wc -w | tr -d ' ' >> ../batches"
            }
        }
    }
}
EOF
    for i in $(seq 12); do touch "f$i.txt"; done
}

@test "batch splits files across jobs with at least 4 in each" {
    export HK_JOBS=8
    batch_config ""
    run hk check --all
    assert_success
    run sort -n ../batches
    assert_output "$(printf '4\n4\n4')"
}

@test "batch_min_files raises the fewest files in a batch" {
    export HK_JOBS=8
    batch_config "batch_min_files = 6"
    run hk check --all
    assert_success
    run sort -n ../batches
    assert_output "$(printf '6\n6')"
}
