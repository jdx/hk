#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_STASH_UNTRACKED=true
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "true" } }
  }
}
EOF
    echo base > f.txt
    echo base > d.txt
    echo a > a.txt
    git add -A
    git commit -qm init
}

teardown() {
    _common_teardown
}

# The index, and the whole worktree as a tree: contents, modes and symlinks.
snapshot() {
    git ls-files -s
    local index="$BATS_TEST_TMPDIR/snapshot-index"
    cp "$(git rev-parse --git-path index)" "$index"
    GIT_INDEX_FILE="$index" git add -A
    GIT_INDEX_FILE="$index" git write-tree
    rm -f "$index"
}

reset_repo() {
    git reset -q --hard
    git clean -qfd
}

@test "stash restores a file reverted to HEAD after a staged edit" {
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo staged >> f.txt
        git add f.txt
        echo base > f.txt
        # Another unstaged change, so that there is something to stash even
        # though f.txt matches HEAD
        echo unstaged >> d.txt
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        reset_repo
    done
}

@test "stash is kept when a step leaves a directory where it has a file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "mkdir -p u.txt && echo output > u.txt/output" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo untracked > u.txt

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_failure
        assert_output --partial "u.txt"
        assert_output --partial "Stash has been preserved"
        # The step's output is intact and the stash still has the file
        run cat u.txt/output
        assert_output output
        run git show 'stash@{0}^3:u.txt'
        assert_output untracked

        git stash drop -q
        rm -rf u.txt
        git reset -q --hard
    done
}

@test "stash is kept when a step writes a file where it has a symlink" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "rm -f link && echo output > link" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        ln -s a.txt link

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_failure
        assert_output --partial "a step changed link"
        assert_output --partial "Stash has been preserved"
        # The step's file is intact and the stash still has the symlink
        run test -L link
        assert_failure
        run cat link
        assert_output output
        run git cat-file -t 'stash@{0}^3:link'
        assert_output blob

        git stash drop -q
        rm -f link
        git reset -q --hard
    done
}

@test "stash restores an unstaged file replaced by an untracked directory" {
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        rm d.txt
        mkdir d.txt
        echo inner > d.txt/inner
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        rm -rf d.txt
        reset_repo
    done
}

@test "a stash of untracked files restores unstaged deletions" {
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        rm d.txt
        echo untracked > untracked.txt
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        reset_repo
    done
}
