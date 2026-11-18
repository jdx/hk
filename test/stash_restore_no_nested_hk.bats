#!/usr/bin/env bats

# Restoring a stashed untracked file uses `git restore`, which runs the
# post-checkout hook. hk must not start a nested hk for it.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "restoring an untracked file does not run the post-checkout hook" {
    MARK="$BATS_TEST_TMPDIR/post-checkout-ran"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["noop"] { check = "true" }
    }
  }
  ["post-checkout"] {
    steps {
      ["marker"] { check = "touch '$MARK'" }
    }
  }
}
PKL
    echo one > a.txt
    git add hk.pkl a.txt
    git commit -q -m base --no-verify
    hk install

    echo two >> a.txt
    git add a.txt
    echo "three (unstaged)" >> a.txt
    echo new > notes.txt

    run git commit -q -m two
    [ "$status" -eq 0 ]
    [ ! -e "$MARK" ]
    [ "$(cat notes.txt)" = "new" ]
    [ "$(tail -n 1 a.txt)" = "three (unstaged)" ]
}
