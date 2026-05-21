#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "pre-commit: staged deletion is preserved when stash is triggered" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["echo-check"] {
        glob = "**/*"
        check = "echo linter ran"
      }
    }
  }
}
PKL

    # Initial commit with files
    echo "file to delete" > delete-me.json
    echo "will be modified" > modify-me.txt
    echo "unstaged file" > unstaged.txt
    git add hk.pkl delete-me.json modify-me.txt unstaged.txt
    git commit -m "initial commit"
    hk install

    # Set up the bug scenario:
    # 1. Stage a file deletion
    # 2. Stage a modification (so hk has files to lint)
    # 3. Have an unstaged change (triggers stash)
    git rm delete-me.json
    echo "staged modification" > modify-me.txt
    git add modify-me.txt
    echo "unstaged change" > unstaged.txt

    # Sanity: the file is gone before the commit
    assert_file_not_exists delete-me.json

    run git commit -m "delete a file"
    assert_success

    # The deleted file should NOT be recreated by pop_stash
    assert_file_not_exists delete-me.json

    # And it should not reappear as an untracked file
    run git status --porcelain
    assert_success
    refute_output --partial "delete-me.json"
}

@test "pre-commit: git rm --cached preserves untracked worktree file when HK_STASH_UNTRACKED=true" {
    export HK_STASH_UNTRACKED=true
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["echo-check"] {
        glob = "**/*"
        check = "echo linter ran"
      }
    }
  }
}
PKL

    echo "tracked content" > rm-cached.txt
    echo "other" > other.txt
    git add hk.pkl rm-cached.txt other.txt
    git commit -m "initial"
    hk install

    # `git rm --cached` stages a deletion but keeps the file on disk as untracked.
    git rm --cached rm-cached.txt
    echo "unstaged change" > other.txt

    # Sanity: the file is still present on disk and untracked
    assert_file_exists rm-cached.txt

    run git commit -m "remove from index"
    assert_success

    # The worktree file must survive — the user only removed it from the index.
    assert_file_exists rm-cached.txt
    run cat rm-cached.txt
    assert_output "tracked content"

    # And it should not be re-added to the index.
    run git ls-files
    refute_output --partial "rm-cached.txt"
}

@test "pre-commit: staged deletion preserved with stash=git and no fixer" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["noop"] {
        glob = "**/*"
        check = "true"
      }
    }
  }
}
PKL

    echo "delete-me" > deleted.txt
    echo "keep" > kept.txt
    git add hk.pkl deleted.txt kept.txt
    git commit -m "initial"
    hk install

    git rm deleted.txt
    echo "unstaged change" > kept.txt

    run git commit -m "remove deleted.txt"
    assert_success

    assert_file_not_exists deleted.txt
    run git ls-files
    assert_success
    refute_output --partial "deleted.txt"
}
