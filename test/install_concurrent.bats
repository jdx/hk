#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "concurrent local installs leave one complete set of config hooks" {
    if ! git version | awk '{split($3,v,"."); exit !(v[1]>2 || (v[1]==2 && v[2]>=54))}'; then
        skip "git 2.54+ required for config-based hooks"
    fi

    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["noop"] { check = "true" } } } }
EOF
    git add hk.pkl
    git commit -m 'add hk config'
    git worktree add -b concurrent-install "$TEST_TEMP_DIR/linked"

    # Both worktrees update the same local Git config. Exercise first
    # installation and replacement of existing entries.
    for round in 1 2; do
        pids=()
        for i in 1 2 3 4 5 6 7 8; do
            if [ $((i % 2)) -eq 0 ]; then
                (cd "$TEST_TEMP_DIR/linked" && hk install --mise) >"$TEST_TEMP_DIR/install-$round-$i.log" 2>&1 &
            else
                hk install --mise >"$TEST_TEMP_DIR/install-$round-$i.log" 2>&1 &
            fi
            pids+=("$!")
        done
        for pid in "${pids[@]}"; do
            wait "$pid" || fail "concurrent hk install failed"
        done

        run git config --local --get-all hook.hk-pre-commit.command
        assert_success
        assert_output 'test "${HK:-1}" = "0" || mise x -- hk run pre-commit --from-hook'

        run git config --local --get-all hook.hk-pre-commit.event
        assert_success
        assert_output pre-commit
    done
}

@test "lock file follows group-shared repository config permissions" {
    if ! git version | awk '{split($3,v,"."); exit !(v[1]>2 || (v[1]==2 && v[2]>=54))}'; then
        skip "git 2.54+ required for config-based hooks"
    fi

    cd "$TEST_TEMP_DIR/src"
    git init --shared=group shared
    cd shared
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["noop"] { check = "true" } } } }
EOF

    umask 0022
    hk install --mise
    lock_path="$(git rev-parse --git-path config).hk-install.lock"
    [ "$(ls -ld "$lock_path" | awk '{print substr($1,6,1)}')" = w ]
}

@test "lock remains writable when Git config is read-only" {
    if ! git version | awk '{split($3,v,"."); exit !(v[1]>2 || (v[1]==2 && v[2]>=54))}'; then
        skip "git 2.54+ required for config-based hooks"
    fi

    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["noop"] { check = "true" } } } }
EOF
    chmod a-w "$(git rev-parse --git-path config)"

    hk install --mise
    lock_path="$(git rev-parse --git-path config).hk-install.lock"
    [ "$(ls -ld "$lock_path" | awk '{print substr($1,3,1)}')" = w ]
    hk install --mise
}
