#!/usr/bin/env bats

# Per-hook `jobs` and `hide_when_done` override the top-level hk.pkl settings
# and yield to the environment, so a developer can still override them.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    # The shared setup pins HK_JOBS, which would outrank the hook's `jobs`.
    unset HK_JOBS
}

teardown() {
    _common_teardown
}

commit_config() {
    git add hk.pkl
    git commit -qm "initial commit"
}

# Each step notes in ../concurrency how many steps were running at once.
max_concurrency() {
    sort -n ../concurrency | tail -1 | tr -d ' '
}

@test "hook jobs limits how many steps run at once" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
jobs = 3
local probe = "mkdir -p ../running; touch ../running/\$\$; sleep 1; ls ../running | wc -l >> ../concurrency; rm ../running/\$\$"
hooks {
    ["check"] {
        jobs = 1
        steps {
            ["a"] { check = probe }
            ["b"] { check = probe }
            ["c"] { check = probe }
        }
    }
}
EOF
    commit_config

    run hk check --all
    assert_success
    assert_equal "$(max_concurrency)" "1"
}

@test "HK_JOBS overrides hook jobs" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
local probe = "mkdir -p ../running; touch ../running/\$\$; sleep 1; ls ../running | wc -l >> ../concurrency; rm ../running/\$\$"
hooks {
    ["check"] {
        jobs = 1
        steps {
            ["a"] { check = probe }
            ["b"] { check = probe }
            ["c"] { check = probe }
        }
    }
}
EOF
    commit_config

    HK_JOBS=3 run hk check --all
    assert_success
    [ "$(max_concurrency)" -gt 1 ]
}

@test "hook hide_when_done hides steps once they succeed" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        hide_when_done = true
        steps { ["greet"] { check = "echo hi" } }
    }
}
EOF
    commit_config

    run hk check --all
    assert_success
    refute_output --partial "✔ greet"

    HK_HIDE_WHEN_DONE=0 run hk check --all
    assert_success
    assert_output --partial "✔ greet"
}

@test "validate warns about hk settings in hook env" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        env { ["HK_JOBS"] = "2" }
        steps { ["greet"] { check = "echo hi" } }
    }
}
EOF
    run hk validate
    assert_success
    assert_output --partial "Hook 'pre-commit' sets HK_JOBS in \`env\`"
    assert_output --partial "set \`jobs\` on the hook instead"
}
