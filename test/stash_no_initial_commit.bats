#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "pre-commit skips stashing when the repository has no commits" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stage = true
    stash = "git"
    steps {
      ["upper"] {
        glob = "*.txt"
        check = "! grep -q bad {{files}}"
        fix = "sed -i.bak s/bad/good/ {{files}} && rm -f *.bak"
      }
    }
  }
}
PKL
    echo bad > a.txt
    git add a.txt
    echo unstaged >> a.txt

    run hk run pre-commit
    assert_success
    # Without a stash, the fixer sees and stages the unstaged edit too.
    assert_equal "$(git show :a.txt)" "$(printf 'good\nunstaged')"
    assert_equal "$(cat a.txt)" "$(printf 'good\nunstaged')"
    run git stash list
    assert_output ""
}
