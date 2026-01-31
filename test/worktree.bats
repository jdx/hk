#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup

    # _common_setup already created a git repo at $TEST_TEMP_DIR/src/proj
    # Create initial commit so we can add worktrees
    echo "init" > file.txt
    git add file.txt
    git commit -m "initial commit"

    # Create a worktree
    WORKTREE_DIR="$TEST_TEMP_DIR/worktree"
    git worktree add "$WORKTREE_DIR" -b test-branch
}

teardown() {
    _common_teardown
}

@test "hk install works in a git worktree" {
    cd "$WORKTREE_DIR"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF

    run hk install
    assert_success
    assert_output --partial "Installed hk hook: "
    assert_output --partial "pre-commit"
}

@test "hk uninstall works in a git worktree" {
    cd "$WORKTREE_DIR"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF

    hk install
    run hk uninstall
    assert_success
    assert_output --partial "removed hook: "
}

@test "hk check works in a git worktree" {
    cd "$WORKTREE_DIR"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["echo"] { check = "echo ok" } } } }
EOF

    run hk check --all
    assert_success
}

@test "hk fix works in a git worktree" {
    cd "$WORKTREE_DIR"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["fix"] { steps { ["echo"] { check = "echo ok" } } } }
EOF

    run hk fix --all
    assert_success
}

@test "pre-commit hook fires in a git worktree" {
    cd "$WORKTREE_DIR"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["linter"] { check = "echo linter-ran" }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "add hk config"

    hk install

    echo "new content" > newfile.txt
    git add newfile.txt
    run git commit -m "test commit"
    assert_success
    assert_output --partial "linter-ran"
}

@test "hk run pre-commit works in a git worktree" {
    cd "$WORKTREE_DIR"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["linter"] { check = "echo linter-ran" }
        }
    }
}
EOF
    echo "staged" > staged.txt
    git add hk.pkl staged.txt

    run hk run pre-commit
    assert_success
    assert_output --partial "linter-ran"
}
