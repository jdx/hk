#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "expr git status is available in conditions" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["staged"]    { check = "echo S > staged.out";    condition = "git.staged_files != []" }
            ["unstaged"]  { check = "echo U > unstaged.out";  condition = "git.unstaged_files != []" }
            ["untracked"] { check = "echo N > untracked.out"; condition = "git.untracked_files != []" }
            ["staged_added"]    { check = "echo SA > staged_added.out";    condition = "git.staged_added_files != []" }
            ["staged_deleted"]  { check = "echo SD > staged_deleted.out";  condition = "git.staged_deleted_files != []" }
            ["staged_modified"] { check = "echo SM > staged_modified.out"; condition = "git.staged_modified_files != []" }
            ["unstaged_modified"] { check = "echo UM > unstaged_modified.out"; condition = "git.unstaged_modified_files != []" }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # create one staged file and one untracked (unstaged) file
    echo staged > a.txt
    git add a.txt
    echo untracked > b.txt

    # prepare a tracked file c.txt and then stage its deletion
    echo tracked > c.txt
    git add c.txt
    git commit c.txt -m "add c"
    git rm -q c.txt

    # prepare a tracked file e.txt and modify it without staging (unstaged modified)
    echo tracked > e.txt
    git add e.txt
    git commit e.txt -m "add e"
    echo mod >> e.txt

    # prepare a tracked file f.txt and stage a modification
    echo tracked > f.txt
    git add f.txt
    git commit f.txt -m "add f"
    echo mod >> f.txt
    git add f.txt

    run hk check -v
    echo "$output"
    [ "$status" -eq 0 ]

    assert_file_exists staged.out
    assert_file_exists unstaged.out
    assert_file_exists untracked.out
    assert_file_exists staged_added.out
    assert_file_exists staged_deleted.out
    assert_file_exists staged_modified.out
    assert_file_exists unstaged_modified.out
}
