#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# This reproduces issue #387 where an untracked file gets accidentally committed
# during pre-commit hook execution. The user has:
# - Staged Python files that get fixed by ruff
# - An untracked test.py file
#
# Expected: only the staged files should be committed
# Bug: the untracked test.py file also gets committed

@test "untracked files should not be committed during pre-commit with fix" {
    # Create a simple pre-commit hook with ruff
    cat <<PKL > hk.pkl
amends "\$PKL_PATH/Config.pkl"
import "\$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["ruff"] = Builtins.ruff
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init hk'
    hk install

    # Create a tracked Python file and commit it
    echo 'def foo(): pass' > staged.py
    git add staged.py
    git commit -m 'add staged.py'

    # Make a change to the tracked file and stage it
    echo 'def foo():
    return 1' > staged.py
    git add staged.py

    # Create an untracked Python file that should NOT be committed
    echo 'def bar(): pass' > test.py

    # Verify the initial state
    run bash -c "git diff --staged --name-only"
    assert_line 'staged.py'

    run bash -c "git ls-files --others --exclude-standard"
    assert_line 'test.py'

    # Run commit - this should only commit staged.py, not test.py
    run git commit -m 'update staged.py'
    assert_success

    # Verify that ONLY staged.py is in the commit, not test.py
    run bash -c "git show HEAD --name-only --pretty=format:"
    assert_line 'staged.py'
    refute_line 'test.py'

    # Verify test.py still exists as untracked
    run bash -c "git ls-files --others --exclude-standard"
    assert_line 'test.py'

    # Verify test.py is not tracked in git
    run bash -c "git ls-files test.py"
    assert_output ""
}

@test "untracked files should not be committed with broad stage glob" {
    # This reproduces the exact scenario from issue #387
    # where the user has stage = "**/*" which might accidentally include untracked files
    cat <<PKL > hk.pkl
amends "\$PKL_PATH/Config.pkl"
import "\$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["ruff"] {
        glob = "*.py"
        stage = "**/*"
        fix = "ruff check --fix"
      }
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init hk'
    hk install

    # Create a tracked Python file
    echo 'def foo(): pass' > staged.py
    git add staged.py
    git commit -m 'add staged.py'

    # Make changes and stage them
    echo 'def foo():
    return 1' > staged.py
    git add staged.py

    # Create an untracked Python file
    echo 'def test(): pass' > test.py

    # Verify initial state
    run bash -c "git diff --staged --name-only"
    assert_line 'staged.py'

    run bash -c "git status --porcelain test.py"
    assert_line '?? test.py'

    # Run commit
    run git commit -m 'update staged.py'
    assert_success

    # test.py should NOT be in the commit
    run bash -c "git show HEAD --name-only --pretty=format:"
    assert_line 'staged.py'
    refute_line 'test.py'

    # test.py should still be untracked
    run bash -c "git status --porcelain test.py"
    assert_line '?? test.py'
}
