setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "conventional commit builtin validates configured scopes" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["commit-msg"] {
    steps {
      ["conventional-commit"] = (Builtins.check_conventional_commit) {
        allowed_scopes = List("web", "api")
      }
    }
  }
}
PKL

    for title in "fix(api): fix auth" "feat(web): add dashboard" "fix(api)!: change auth" "fix: fix auth"; do
        printf '%s\n' "$title" > "commit message.txt"
        run hk run commit-msg "commit message.txt"
        assert_success
    done

    printf '%s\n' "fix(other): fix auth" > "commit message.txt"
    run hk run commit-msg "commit message.txt"
    assert_failure
    assert_output --partial "Invalid commit scope: 'other'. Allowed scopes: web, api"
}

@test "conventional commit builtin with empty scopes allows only unscoped commits" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["commit-msg"] {
    steps {
      ["conventional-commit"] = (Builtins.check_conventional_commit) {
        allowed_scopes = List()
      }
    }
  }
}
PKL

    printf '%s\n' "fix: fix auth" > message.txt
    run hk run commit-msg message.txt
    assert_success

    printf '%s\n' "fix(api): fix auth" > message.txt
    run hk run commit-msg message.txt
    assert_failure
    assert_output --partial "Invalid commit scope: 'api'"
}

@test "commit-msg hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks = new {
    ["commit-msg"] {
        steps {
            ["validate-commit-msg"] {
                check = "grep -q '^feat: ' {{commit_msg_file}} || (echo 'Commit message must start with feat:' >&2 && exit 1)"
            }
        }
    }
}
EOF
    hk install
    echo "test" > test.txt
    git add test.txt
    run git commit -m "test"
    assert_failure
    assert_output --partial "Commit message must start with feat:"

    run git commit -m "feat: add test file"
    assert_success
}

@test "commit-msg hook in worktree" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks = new {
    ["commit-msg"] {
        steps {
            ["validate-commit-msg"] {
                check = "grep -q '^feat: ' {{commit_msg_file}} || (echo 'Commit message must start with feat:' >&2 && exit 1)"
            }
        }
    }
}
EOF
    # Need an initial commit before creating a worktree
    echo "init" > init.txt
    git add init.txt
    git commit -m "feat: initial commit"

    hk install

    # Create a worktree
    git worktree add ../worktree -b test-branch
    cp hk.pkl ../worktree/hk.pkl
    cd ../worktree

    echo "test" > test.txt
    git add test.txt
    run git commit -m "test"
    assert_failure
    assert_output --partial "Commit message must start with feat:"

    run git commit -m "feat: add test file"
    assert_success
}
