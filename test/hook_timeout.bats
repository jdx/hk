#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hook timeout stops a slow step and fails the hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        timeout = "1s"
        steps {
            ["slow"] {
                check = "sleep 30"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    start=$SECONDS
    run hk check --all
    assert_failure
    assert_output --partial "timed out after 1s"
    [ $((SECONDS - start)) -lt 15 ]
}

@test "hook timeout leaves a hook that finishes in time alone" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        timeout = "30s"
        steps {
            ["quick"] {
                check = "true"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    run hk check --all
    assert_success
    refute_output --partial "timed out"
}

@test "hook timeout restores stashed changes" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        timeout = "1s"
        steps {
            ["slow"] {
                glob = "*.txt"
                fix = "sleep 30"
            }
        }
    }
}
EOF
    echo base > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit" --no-verify
    echo staged-change > file.txt
    git add file.txt
    echo unstaged-change >> file.txt

    run hk run pre-commit
    assert_failure
    assert_output --partial "timed out after 1s"
    run cat file.txt
    assert_output --partial "unstaged-change"
}

@test "hook timeout also bounds the report command" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        timeout = "1s"
        report = "sleep 30"
        steps {
            ["quick"] {
                check = "true"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    start=$SECONDS
    run hk check --all
    assert_failure
    assert_output --partial "timed out after 1s"
    [ $((SECONDS - start)) -lt 15 ]
}

@test "hook timeout rejects values without a unit" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        timeout = "10"
        steps {
            ["quick"] {
                check = "true"
            }
        }
    }
}
EOF
    run hk validate
    assert_failure
}
