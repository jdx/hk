#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export STASH_PROBE_FILE=tracked.txt
    export STASH_PROBE_EXIT=0
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    stage = false
    fix = false
    steps { ["probe"] { check = "bash probe.sh {{files}}" } }
  }
}
EOF
    cat > probe.sh <<'EOF'
set -eu
test "$(cat "$STASH_PROBE_FILE")" = indexed
test "$(cat other.txt)" = staged
test "$(cat extra.txt)" = baseline
test "$#" -eq 1
test "$1" = other.txt
echo "indexed contents visible"
exit "$STASH_PROBE_EXIT"
EOF
    printf 'indexed\n' > tracked.txt
    printf 'baseline\n' > other.txt
    printf 'baseline\n' > extra.txt
    git add .
    git commit -qm baseline
}

teardown() {
    _common_teardown
}

snapshot() {
    git ls-files --stage
    git diff --cached --binary
    git diff --binary
    git status --porcelain=v1
    git for-each-ref refs/stash refs/hk/stash
}

@test "stash isolates unstaged deletions and restores them after passing and failing checks" {
    for scenario in deletion-only deletion-and-edit added-then-deleted; do
        for untracked in false true; do
            for hook_exit in 0 23; do
                export STASH_PROBE_FILE=tracked.txt
                export STASH_PROBE_EXIT=$hook_exit
                printf 'staged\n' > other.txt
                git add other.txt
                if [ "$scenario" = added-then-deleted ]; then
                    export STASH_PROBE_FILE=added.txt
                    printf 'indexed\n' > added.txt
                    git add added.txt
                fi
                rm "$STASH_PROBE_FILE"
                if [ "$scenario" = deletion-and-edit ]; then
                    printf 'unstaged\n' > extra.txt
                fi
                before=$(snapshot)

                HK_STASH_UNTRACKED=$untracked run hk run pre-commit --check -v
                assert_output --partial "indexed contents visible"
                if [ "$hook_exit" = 0 ]; then
                    assert_success
                else
                    assert_failure
                fi
                assert_file_not_exists "$STASH_PROBE_FILE"
                assert_equal "$(snapshot)" "$before"
                git reset -q --hard
            done
        done
    done
}

@test "stash none leaves unstaged deletions and edits visible" {
    cat > probe.sh <<'EOF'
set -eu
test ! -e tracked.txt
test "$(cat extra.txt)" = unstaged
echo "unstaged changes visible"
EOF
    printf 'staged\n' > other.txt
    git add probe.sh other.txt
    rm tracked.txt
    printf 'unstaged\n' > extra.txt
    before=$(snapshot)

    run hk run pre-commit --check --stash none -v
    assert_success
    assert_output --partial "unstaged changes visible"
    assert_equal "$(snapshot)" "$before"
}

@test "stash does not treat sparse-checkout entries as unstaged deletions" {
    mkdir included excluded
    printf 'excluded\n' > excluded/input.txt
    git add excluded
    git commit -qm sparse
    git sparse-checkout set --cone included
    printf 'staged\n' > other.txt
    git add other.txt
    before=$(snapshot)

    run hk run pre-commit --check -v
    assert_success
    assert_output --partial "No unstaged changes to stash"
    assert_file_not_exists excluded/input.txt
    assert_equal "$(snapshot)" "$before"
}
