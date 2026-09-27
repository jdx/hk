#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    for i in $(seq 40); do
        touch "f$i.txt" "f$i.md"
    done
}

teardown() {
    _common_teardown
}

# Each batch appends one line with its step and file count.

@test "batched steps that run together share the jobs" {
    export HK_JOBS=4
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["txt"] {
                glob = "*.txt"
                batch = true
                check = "echo txt \$(echo {{ files }} | wc -w) >> ../batches"
            }
            ["md"] {
                glob = "*.md"
                batch = true
                check = "echo md \$(echo {{ files }} | wc -w) >> ../batches"
            }
        }
    }
}
EOF
    run hk check --all
    assert_success
    run sort ../batches
    assert_output "$(printf 'md 20\nmd 20\ntxt 20\ntxt 20')"
}

@test "a batched step alone gets every job" {
    export HK_JOBS=4
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["txt"] {
                glob = "*.txt"
                batch = true
                check = "echo txt \$(echo {{ files }} | wc -w) >> ../batches"
            }
            ["md"] {
                glob = "*.md"
                check = "echo md \$(echo {{ files }} | wc -w) >> ../batches"
            }
        }
    }
}
EOF
    run hk check --all
    assert_success
    run sort ../batches
    assert_output "$(printf 'md 40\ntxt 10\ntxt 10\ntxt 10\ntxt 10')"
}

@test "a batched step that depends on another keeps every job" {
    export HK_JOBS=4
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["txt"] {
                glob = "*.txt"
                batch = true
                check = "echo txt \$(echo {{ files }} | wc -w) >> ../batches"
            }
            ["md"] {
                glob = "*.md"
                batch = true
                depends = "txt"
                check = "echo md \$(echo {{ files }} | wc -w) >> ../batches"
            }
        }
    }
}
EOF
    run hk check --all
    assert_success
    run sort ../batches
    assert_output "$(printf 'md 10\nmd 10\nmd 10\nmd 10\ntxt 10\ntxt 10\ntxt 10\ntxt 10')"
}

@test "a batched step skipped for a missing required variable takes no share" {
    export HK_JOBS=4
    unset HK_TEST_UNSET_VAR
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["txt"] {
                glob = "*.txt"
                batch = true
                check = "echo txt \$(echo {{ files }} | wc -w) >> ../batches"
            }
            ["md"] {
                glob = "*.md"
                batch = true
                required = List("HK_TEST_UNSET_VAR")
                check = "echo md \$(echo {{ files }} | wc -w) >> ../batches"
            }
        }
    }
}
EOF
    run hk check --all
    assert_success
    run sort ../batches
    assert_output "$(printf 'txt 10\ntxt 10\ntxt 10\ntxt 10')"
}

@test "a batched step whose files are all binary takes no share" {
    export HK_JOBS=4
    for i in $(seq 40); do printf 'a\0b' > "f$i.md"; done
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["txt"] {
                glob = "*.txt"
                batch = true
                check = "echo txt \$(echo {{ files }} | wc -w) >> ../batches"
            }
            ["md"] {
                glob = "*.md"
                batch = true
                check = "echo md \$(echo {{ files }} | wc -w) >> ../batches"
            }
        }
    }
}
EOF
    run hk check --all
    assert_success
    run sort ../batches
    assert_output "$(printf 'txt 10\ntxt 10\ntxt 10\ntxt 10')"
}

@test "each step group shares the jobs only among its own steps" {
    export HK_JOBS=4
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["txt"] {
                glob = "*.txt"
                batch = true
                check = "echo txt \$(echo {{ files }} | wc -w) >> ../batches"
            }
            ["md"] {
                glob = "*.md"
                batch = true
                exclusive = true
                check = "echo md \$(echo {{ files }} | wc -w) >> ../batches"
            }
        }
    }
}
EOF
    run hk check --all
    assert_success
    run sort ../batches
    assert_output "$(printf 'md 10\nmd 10\nmd 10\nmd 10\ntxt 10\ntxt 10\ntxt 10\ntxt 10')"
}
