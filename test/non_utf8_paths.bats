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
