#!/usr/bin/env bats

# Untracked files must be stashed even when tracked files also have unstaged
# changes, which used to limit the stash to the tracked paths and leave every
# untracked file in the worktree while steps ran.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_STASH_UNTRACKED=true
    echo a > a.txt
    echo base > f.txt
    echo base > d.txt
    printf '#!/bin/sh\n' > x.sh
    git add -A
    git commit -qm init
}

teardown() {
    _common_teardown
}

hook_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "$1" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
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

# A staged edit, unstaged changes to tracked files, and untracked files whose
# bytes are easy to get wrong: no final newline, CRLF, binary, an executable
# and a symlink, some in an untracked directory.
make_changes() {
    echo staged >> a.txt
    git add a.txt
    echo unstaged >> f.txt
    rm d.txt
    chmod +x x.sh
    printf 'user data\r\nno final newline' > u.txt
    mkdir -p new/deep
    printf '\x00\x01\xfe\xff binary\n' > new/deep/n.bin
    printf '#!/bin/sh\necho mine\n' > new/run.sh
    chmod +x new/run.sh
    ln -s u.txt link
    mkdir -p "$BATS_TEST_TMPDIR/orig"
    cp -a u.txt new link "$BATS_TEST_TMPDIR/orig/"
}

@test "steps do not see untracked files when tracked files have unstaged changes" {
    local seen="$BATS_TEST_TMPDIR/seen.txt"
    hook_config 'for p in u.txt link new f.txt; do if test -e $p || test -L $p; then echo $p; fi; done > '"'$seen'"'; cat f.txt >> '"'$seen'"
    for libgit2 in 1 0; do
        make_changes
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success

        # The steps saw only the tracked file, without its unstaged edit
        run cat "$BATS_TEST_TMPDIR/seen.txt"
        assert_output $'f.txt\nbase'
        # Every untracked file is back byte for byte, with its mode, and the
        # unstaged changes to tracked files are back too
        cmp u.txt "$BATS_TEST_TMPDIR/orig/u.txt"
        cmp new/deep/n.bin "$BATS_TEST_TMPDIR/orig/new/deep/n.bin"
        cmp new/run.sh "$BATS_TEST_TMPDIR/orig/new/run.sh"
        assert_file_executable new/run.sh
        assert_equal "$(readlink link)" u.txt
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""

        reset_repo
        rm -rf "$BATS_TEST_TMPDIR/orig" "$BATS_TEST_TMPDIR/seen.txt"
    done
}

@test "a step that overwrites an untracked file cannot destroy it when tracked files have unstaged changes" {
    hook_config "printf step > u.txt"
    for libgit2 in 1 0; do
        make_changes

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_failure
        assert_output --partial "Did not restore u.txt from the stash"
        assert_output --partial "Stash has been preserved"
        # The step's file is kept, every other change is back, and the stash
        # still has the user's file byte for byte
        run cat u.txt
        assert_output step
        cmp new/deep/n.bin "$BATS_TEST_TMPDIR/orig/new/deep/n.bin"
        run cat f.txt
        assert_output $'base\nunstaged'
        assert_file_not_exists d.txt
        git restore --source='stash@{0}^3' -- u.txt
        cmp u.txt "$BATS_TEST_TMPDIR/orig/u.txt"

        git stash drop -q
        reset_repo
        rm -rf "$BATS_TEST_TMPDIR/orig"
    done
}

@test "a fixer that stages everything does not commit untracked files when tracked files have unstaged changes" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["fmt"] {
        glob = "a.txt"
        stage = "**/*"
        fix = "sed -i.bak s/staged/FIXED/ a.txt && rm a.txt.bak"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    hk install
    for libgit2 in 1 0; do
        make_changes

        HK_LIBGIT2=$libgit2 run git commit -qm "commit $libgit2"
        assert_success
        run git show --name-only --format= HEAD
        assert_output a.txt
        run git show HEAD:a.txt
        assert_output $'a\nFIXED'
        cmp u.txt "$BATS_TEST_TMPDIR/orig/u.txt"
        cmp new/deep/n.bin "$BATS_TEST_TMPDIR/orig/new/deep/n.bin"
        run cat f.txt
        assert_output $'base\nunstaged'
        assert_equal "$(git stash list)" ""

        reset_repo
        git reset -q --hard HEAD~1
        rm -rf "$BATS_TEST_TMPDIR/orig"
    done
}

@test "stashing untracked files leaves an intent-to-add entry in the index" {
    hook_config "true"
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo unstaged >> f.txt
        echo new > ita.txt
        git add --intent-to-add ita.txt
        echo untracked > u.txt
        before=$(git ls-files --stage; git status --porcelain=v2)

        # git refuses to stash an intent-to-add entry. Whatever the hook's
        # outcome, the index must still mark ita.txt as intent-to-add
        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_equal "$(git ls-files --stage; git status --porcelain=v2)" "$before"
        run git diff --cached --name-only
        assert_output a.txt
        run cat u.txt
        assert_output untracked
        assert_equal "$(git stash list)" ""

        git rm -q --cached ita.txt
        reset_repo
    done
}
