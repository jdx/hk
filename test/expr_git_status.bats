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
    ["fix"] {
        steps {
            ["staged"]    { fix = "echo S > staged.out";    condition = "git.staged_files != []" }
            ["unstaged"]  { fix = "echo U > unstaged.out";  condition = "git.unstaged_files != []" }
            ["untracked"] { fix = "echo N > untracked.out"; condition = "git.untracked_files != []" }
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

    run hk fix -v
    echo "$output"
    [ "$status" -eq 0 ]

    assert_file_exists staged.out
    assert_file_exists unstaged.out
    assert_file_exists untracked.out
}
