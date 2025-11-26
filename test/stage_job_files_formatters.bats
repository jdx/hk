#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# These tests validate that formatters with stage=<JOB_FILES> and check_list_files
# only stage files that actually need formatting, not all files matching the glob.
# This prevents untracked files from being accidentally staged.

@test "ruff with <JOB_FILES> only stages files with linting issues" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["ruff"] = Builtins.ruff
    }
  }
}
EOF
    git add hk.pkl
    git commit -m 'init'
    hk install

    # Create tracked file with linting issue
    echo 'import sys
print("hello")' > has_issue.py
    git add has_issue.py
    git commit -m 'add has_issue.py'

    # Create tracked file without issues
    echo 'print("world")' > no_issue.py
    git add no_issue.py
    git commit -m 'add no_issue.py'

    # Modify file with issue and stage it
    echo 'import sys
import os
print("goodbye")' > has_issue.py
    git add has_issue.py

    # Modify file without issue but don't stage
    echo 'print("goodbye")' > no_issue.py

    # Create untracked file with issue
    echo 'import sys
print("test")' > untracked.py

    # Run pre-commit
    run git commit -m 'test'
    assert_success

    # Only has_issue.py should be in commit (ruff fixed the imports)
    run bash -c "git show HEAD --name-only --pretty=format:"
    assert_line 'has_issue.py'
    refute_line 'no_issue.py'
    refute_line 'untracked.py'

    # untracked.py should still be untracked
    run bash -c "git status --porcelain untracked.py"
    assert_line '?? untracked.py'
}

@test "ruff_format with <JOB_FILES> only stages files needing formatting" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["ruff_format"] = Builtins.ruff_format
    }
  }
}
EOF
    git add hk.pkl
    git commit -m 'init'
    hk install

    # Create tracked file needing formatting
    echo 'x=1+2' > needs_format.py
    git add needs_format.py
    git commit -m 'add needs_format.py'

    # Create tracked file already formatted
    echo 'x = 1 + 2' > already_formatted.py
    git add already_formatted.py
    git commit -m 'add already_formatted.py'

    # Modify file needing formatting and stage it
    echo 'y=3+4' > needs_format.py
    git add needs_format.py

    # Modify formatted file but don't stage
    echo 'y = 5 + 6' > already_formatted.py

    # Create untracked file needing formatting
    echo 'z=7+8' > untracked.py

    # Run pre-commit
    run git commit -m 'test'
    assert_success

    # Only needs_format.py should be in commit (formatted)
    run bash -c "git show HEAD --name-only --pretty=format:"
    assert_line 'needs_format.py'
    refute_line 'already_formatted.py'
    refute_line 'untracked.py'

    # untracked.py should still be untracked
    run bash -c "git status --porcelain untracked.py"
    assert_line '?? untracked.py'
}

@test "black with <JOB_FILES> only stages files needing formatting" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["black"] = Builtins.black
    }
  }
}
EOF
    git add hk.pkl
    git commit -m 'init'
    hk install

    # Create tracked file needing formatting
    echo 'x=1+2' > needs_format.py
    git add needs_format.py
    git commit -m 'add needs_format.py'

    # Create tracked file already formatted
    echo 'x = 1 + 2' > already_formatted.py
    git add already_formatted.py
    git commit -m 'add already_formatted.py'

    # Modify file needing formatting and stage it
    echo 'y=3+4' > needs_format.py
    git add needs_format.py

    # Modify formatted file but don't stage
    echo 'y = 5 + 6' > already_formatted.py

    # Create untracked file needing formatting
    echo 'z=7+8' > untracked.py

    # Run pre-commit
    run git commit -m 'test'
    assert_success

    # Only needs_format.py should be in commit (formatted by black)
    run bash -c "git show HEAD --name-only --pretty=format:"
    assert_line 'needs_format.py'
    refute_line 'already_formatted.py'
    refute_line 'untracked.py'

    # untracked.py should still be untracked
    run bash -c "git status --porcelain untracked.py"
    assert_line '?? untracked.py'
}

@test "prettier with <JOB_FILES> only stages files needing formatting" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["prettier"] = Builtins.prettier
    }
  }
}
EOF
    git add hk.pkl
    git commit -m 'init'
    hk install

    # Create tracked file needing formatting
    echo '{"a":1}' > needs_format.json
    git add needs_format.json
    git commit -m 'add needs_format.json'

    # Create tracked file already formatted
    echo '{
  "a": 1
}' > already_formatted.json
    git add already_formatted.json
    git commit -m 'add already_formatted.json'

    # Modify file needing formatting and stage it
    echo '{"b":2}' > needs_format.json
    git add needs_format.json

    # Modify formatted file but don't stage
    echo '{
  "b": 2
}' > already_formatted.json

    # Create untracked file needing formatting
    echo '{"c":3}' > untracked.json

    # Run pre-commit
    run git commit -m 'test'
    assert_success

    # Only needs_format.json should be in commit (formatted by prettier)
    run bash -c "git show HEAD --name-only --pretty=format:"
    assert_line 'needs_format.json'
    refute_line 'already_formatted.json'
    refute_line 'untracked.json'

    # untracked.json should still be untracked
    run bash -c "git status --porcelain untracked.json"
    assert_line '?? untracked.json'
}
