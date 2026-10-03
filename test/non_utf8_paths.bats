#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    # Some filesystems, such as APFS, reject names that are not valid UTF-8
    if ! touch $'probe\xff' 2>/dev/null; then
        skip "filesystem rejects names that are not valid UTF-8"
    fi
    rm -f $'probe\xff'
    export NO_COLOR=1
}

teardown() {
    _common_teardown
}

@test "non-UTF-8 paths are skipped with a warning and the hook checks the rest" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    steps {
      ["list"] { check = "printf '%s\n' {{files}} > '$BATS_TEST_TMPDIR/files.txt'" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -qm init
    echo a > a.txt
    echo staged > $'staged\xff.txt'
    git add a.txt $'staged\xff.txt'
    echo untracked > $'untracked\xfe.txt'

    for libgit2 in 1 0; do
        rm -f "$BATS_TEST_TMPDIR/files.txt"
        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_output --partial 'WARN  skipped "staged'
        assert_output --partial 'because hk cannot handle paths that are not valid UTF-8'
        run cat "$BATS_TEST_TMPDIR/files.txt"
        assert_output "a.txt"
    done
}

# Runs `hk <args>` on the backend named by $1, then checks that it warned about
# the non-UTF-8 names, did not panic, and gave the step exactly the files in $2
# (newline-separated).
assert_skips_non_utf8() {
    local libgit2=$1 expected=$2
    shift 2
    rm -f "$BATS_TEST_TMPDIR/files.txt"
    HK_LIBGIT2=$libgit2 run hk "$@"
    assert_success
    assert_output --partial 'WARN  skipped "'
    assert_output --partial 'because hk cannot handle paths that are not valid UTF-8'
    refute_output --partial 'panicked'
    run cat "$BATS_TEST_TMPDIR/files.txt"
    assert_output "$expected"
}

@test "non-UTF-8 tracked paths are skipped with a warning however files are selected" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["list"] { glob = List("*.txt"); check = "printf '%s\n' {{files}} > '$BATS_TEST_TMPDIR/files.txt'" }
    }
  }
  ["pre-push"] {
    steps {
      ["list"] { glob = List("*.txt"); check = "printf '%s\n' {{files}} > '$BATS_TEST_TMPDIR/files.txt'" }
    }
  }
}
EOF
    echo base > base.txt
    git add base.txt hk.pkl
    git commit -qm base
    echo a > a.txt
    echo tracked > $'tracked\xff.txt'
    echo later > $'later\xfe.txt'
    git add a.txt $'tracked\xff.txt' $'later\xfe.txt'
    git commit -qm "add files"

    for libgit2 in 1 0; do
        assert_skips_non_utf8 $libgit2 $'a.txt\nbase.txt' check --all
        assert_skips_non_utf8 $libgit2 $'a.txt\nbase.txt' check --glob '*.txt'
        assert_skips_non_utf8 $libgit2 a.txt check --from-ref HEAD~1
        # A from-ref git cannot resolve lists every file at the to-ref
        assert_skips_non_utf8 $libgit2 $'a.txt\nbase.txt' check --from-ref no-such-ref
        assert_skips_non_utf8 $libgit2 a.txt check a.txt $'tracked\xff.txt' $'later\xfe.txt'
        # pre-push receives the pushed range as from and to refs
        assert_skips_non_utf8 $libgit2 a.txt run pre-push --from-ref HEAD~1 --to-ref HEAD
    done
}

@test "stash sets aside non-UTF-8 unstaged and untracked files while steps run" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["probe"] {
        check = "cat -- * > '$BATS_TEST_TMPDIR/seen.txt' 2>/dev/null; true"
      }
    }
  }
}
EOF
    echo a > a.txt
    echo committed > $'tracked\xff.txt'
    git add -A
    git commit -qm init

    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo unstaged >> $'tracked\xff.txt'
        echo untracked > $'untracked\xfe.txt'
        before=$(git status --porcelain=v2 -z | od -c)

        HK_LIBGIT2=$libgit2 HK_STASH_UNTRACKED=true run hk run pre-commit
        assert_success
        assert_output --partial 'because hk cannot handle paths that are not valid UTF-8'

        # The steps saw neither change
        run cat "$BATS_TEST_TMPDIR/seen.txt"
        refute_output --partial unstaged
        refute_output --partial untracked
        # Both changes are back and the stash is gone
        run cat $'tracked\xff.txt'
        assert_output $'committed\nunstaged'
        run cat $'untracked\xfe.txt'
        assert_output untracked
        assert_equal "$(git status --porcelain=v2 -z | od -c)" "$before"
        assert_equal "$(git stash list)" ""

        git commit -qm "commit $libgit2"
        git checkout -q -- .
        rm -f $'untracked\xfe.txt'
    done
}

# The index, and the whole worktree as a tree: contents, modes and symlinks.
snapshot() {
    git ls-files -s -z | od -c
    local index="$BATS_TEST_TMPDIR/snapshot-index"
    cp "$(git rev-parse --git-path index)" "$index"
    GIT_INDEX_FILE="$index" git add -A
    GIT_INDEX_FILE="$index" git write-tree
    rm -f "$index"
}

stash_config() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "true" } }
  }
}
PKL
    echo a > a.txt
    echo base > f.txt
    echo base > d.txt
    echo base > $'bad\xff.txt'
    git add -A
    git commit -qm init
}

@test "stash restores a non-UTF-8 file reverted to HEAD after a staged edit" {
    stash_config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo staged >> $'bad\xff.txt'
        git add $'bad\xff.txt'
        echo base > $'bad\xff.txt'
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 HK_STASH_UNTRACKED=true run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        git reset -q --hard
    done
}

@test "stash restores a deletion, a non-UTF-8 untracked file and a reverted staged edit" {
    stash_config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        rm d.txt
        echo untracked > $'untracked\xfe.txt'
        echo staged >> f.txt
        git add f.txt
        echo base > f.txt
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 HK_STASH_UNTRACKED=true run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        git reset -q --hard
        git clean -qfd
    done
}
