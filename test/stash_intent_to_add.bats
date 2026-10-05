#!/usr/bin/env bats

# `git stash push` refuses to run while the index has an intent-to-add entry
# (`git add -N`), so stashing used to fail before any step ran.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    SEEN="$TEST_TEMP_DIR/seen"
    mkdir -p "$SEEN"
}

teardown() {
    _common_teardown
}

# Writes an hk.pkl whose pre-commit hook records what its step sees and fixes
# staged.txt.
write_config() {
    local stash_method="$1"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "$stash_method"
    steps {
      ["record"] {
        glob = "**/*"
        fix = "printf '%s\\\\n' {{files}} > '$SEEN/files'; ls -A > '$SEEN/worktree'; cp other.txt '$SEEN/other.txt'; printf 'fixed\\\\n' > staged.txt"
      }
    }
  }
}
PKL
}

# Commits the files the intent-to-add cases start from.
commit_initial_files() {
    printf 'old\n' > old.txt
    printf 'staged v1\n' > staged.txt
    printf 'other v1\n' > other.txt
    git add hk.pkl old.txt staged.txt other.txt
    git commit -m "init"
}

assert_intent_to_add_set_aside_and_restored() {
    local use_libgit2="$1" stash_method="$2"
    write_config "$stash_method"
    commit_initial_files

    # A new file, executable, with CRLF, a NUL byte and no final newline
    printf 'first\r\n\0\377last' > new.sh
    chmod +x new.sh
    git add -N new.sh
    # A move whose new path is intent-to-add, which git reports as a
    # worktree rename
    mv old.txt moved.txt
    git add -N moved.txt
    # A staged change for the step, and an unstaged one to stash
    printf 'staged v2\n' > staged.txt
    git add staged.txt
    printf 'other v2\n' > other.txt
    cp -p new.sh "$TEST_TEMP_DIR/new.sh.orig"
    cp -p moved.txt "$TEST_TEMP_DIR/moved.txt.orig"

    run env HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_success

    # The step saw only the staged file, and none of the unstaged content
    run cat "$SEEN/files"
    assert_output "staged.txt"
    run cat "$SEEN/worktree"
    refute_output --partial "new.sh"
    refute_output --partial "moved.txt"
    run cat "$SEEN/other.txt"
    assert_output "other v1"

    # The files came back byte for byte, still executable
    cmp new.sh "$TEST_TEMP_DIR/new.sh.orig"
    cmp moved.txt "$TEST_TEMP_DIR/moved.txt.orig"
    assert [ -x new.sh ]
    run cat other.txt
    assert_output "other v2"

    # They are intent-to-add again, with nothing staged
    run git diff --name-only --no-renames --diff-filter=A
    assert_output "$(printf 'moved.txt\nnew.sh')"
    run git diff --cached --name-only
    assert_output "staged.txt"
    run git show :staged.txt
    assert_output "fixed"
    run git status --porcelain
    assert_output "$(printf ' R old.txt -> moved.txt\n A new.sh\n M other.txt\nM  staged.txt')"

    run git stash list
    assert_output ""
}

@test "stash=git sets intent-to-add files aside with libgit2" {
    assert_intent_to_add_set_aside_and_restored 1 git
}

@test "stash=git sets intent-to-add files aside with the git CLI" {
    assert_intent_to_add_set_aside_and_restored 0 git
}

@test "stash=patch-file sets intent-to-add files aside with libgit2" {
    assert_intent_to_add_set_aside_and_restored 1 patch-file
}

@test "stash=patch-file sets intent-to-add files aside with the git CLI" {
    assert_intent_to_add_set_aside_and_restored 0 patch-file
}

@test "stash sets aside intent-to-add files that are the only unstaged change" {
    write_config git
    commit_initial_files
    printf 'new\n' > new.txt
    git add -N new.txt
    printf 'staged v2\n' > staged.txt
    git add staged.txt

    run hk run pre-commit
    assert_success
    run cat "$SEEN/worktree"
    refute_output --partial "new.txt"
    run cat new.txt
    assert_output "new"
    run git status --porcelain
    assert_output "$(printf ' A new.txt\nM  staged.txt')"
    run git stash list
    assert_output ""
}

@test "stash sets aside intent-to-add files with HK_STASH_UNTRACKED=false" {
    export HK_STASH_UNTRACKED=false
    write_config git
    commit_initial_files
    printf 'new\n' > new.txt
    git add -N new.txt
    printf 'staged v2\n' > staged.txt
    git add staged.txt
    printf 'other v2\n' > other.txt

    run hk run pre-commit
    assert_success
    run cat "$SEEN/worktree"
    refute_output --partial "new.txt"
    run cat new.txt
    assert_output "new"
    run git status --porcelain
    assert_output "$(printf ' A new.txt\n M other.txt\nM  staged.txt')"
    run git stash list
    assert_output ""
}

@test "stash keeps the staged deletion under an intent-to-add file" {
    write_config git
    commit_initial_files
    # Deleting old.txt is staged, and its replacement is intent-to-add
    git rm --cached old.txt
    printf 'replacement\n' > old.txt
    git add -N old.txt
    printf 'staged v2\n' > staged.txt
    git add staged.txt

    run hk run pre-commit
    assert_success
    run cat old.txt
    assert_output "replacement"
    run git status --porcelain
    assert_output "$(printf 'DA old.txt\nM  staged.txt')"
    run git stash list
    assert_output ""
}

@test "git commit leaves intent-to-add files out of the commit" {
    write_config git
    commit_initial_files
    hk install
    printf 'new\n' > new.txt
    git add -N new.txt
    printf 'staged v2\n' > staged.txt
    git add staged.txt

    run git commit -m "change staged.txt"
    assert_success
    run git show --name-only --format= HEAD
    assert_output "staged.txt"
    run git show HEAD:staged.txt
    assert_output "fixed"
    run git status --porcelain
    assert_output " A new.txt"
    run cat new.txt
    assert_output "new"
}

@test "stash keeps intent-to-add contents when a step recreates the file" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["create"] {
        glob = "**/*"
        check = "printf 'from step\\\\n' > new.txt"
      }
    }
  }
}
PKL
    commit_initial_files
    printf 'mine\n' > new.txt
    git add -N new.txt
    printf 'staged v2\n' > staged.txt
    git add staged.txt

    run hk run pre-commit
    assert_failure
    assert_output --partial "their contents before the hook are kept in stash@{0}"
    run cat new.txt
    assert_output "from step"
    run git status --porcelain
    assert_output "$(printf ' A new.txt\nM  staged.txt')"
    run git stash list --format=%gs
    assert_output --regexp "^hk: [0-9]+-[0-9a-f]+-[0-9]+ \(intent-to-add files\)$"
    run git show 'stash@{0}^3:new.txt'
    assert_output "mine"
}

assert_empty_intent_to_add_set_aside() {
    local use_libgit2="$1"
    write_config git
    commit_initial_files
    : > empty.txt
    git add -N empty.txt
    printf 'staged v2\n' > staged.txt
    git add staged.txt
    printf 'other v2\n' > other.txt

    run env HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_success
    run cat "$SEEN/files"
    assert_output "staged.txt"
    run cat "$SEEN/worktree"
    refute_output --partial "empty.txt"
    assert [ -f empty.txt ] && [ ! -s empty.txt ]
    run git status --porcelain
    assert_output "$(printf ' A empty.txt\n M other.txt\nM  staged.txt')"
    run git stash list
    assert_output ""
}

@test "stash sets aside empty intent-to-add files with libgit2" {
    assert_empty_intent_to_add_set_aside 1
}

@test "stash sets aside empty intent-to-add files with the git CLI" {
    assert_empty_intent_to_add_set_aside 0
}

assert_lone_empty_intent_to_add_set_aside() {
    local use_libgit2="$1"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["append"] {
        glob = "**/*.txt"
        fix = "printf '%s\\\\n' {{files}} > '$SEEN/files'; for f in {{files}}; do echo appended >> \$f; done"
      }
    }
  }
}
PKL
    commit_initial_files
    # An empty intent-to-add file, the only change besides a staged one
    : > empty.txt
    git add -N empty.txt
    printf 'staged v2\n' > staged.txt
    git add staged.txt

    run env HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_success
    run cat "$SEEN/files"
    assert_output "staged.txt"
    assert [ -f empty.txt ] && [ ! -s empty.txt ]
    run git status --porcelain
    assert_output "$(printf ' A empty.txt\nM  staged.txt')"
    run git stash list
    assert_output ""
}

@test "stash sets aside a lone empty intent-to-add file with libgit2" {
    assert_lone_empty_intent_to_add_set_aside 1
}

@test "stash sets aside a lone empty intent-to-add file with the git CLI" {
    assert_lone_empty_intent_to_add_set_aside 0
}

assert_non_utf8_intent_to_add_set_aside() {
    local use_libgit2="$1"
    # Some filesystems, such as APFS, reject names that are not valid UTF-8
    if ! touch $'probe\xff' 2>/dev/null; then
        skip "filesystem rejects names that are not valid UTF-8"
    fi
    rm -f $'probe\xff'
    write_config git
    commit_initial_files
    printf 'first\r\n\0last' > $'bad\xffname'
    git add -N $'bad\xffname'
    cp $'bad\xffname' "$TEST_TEMP_DIR/bad.orig"
    printf 'staged v2\n' > staged.txt
    git add staged.txt

    run env HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_success
    run cat "$SEEN/files"
    assert_output "staged.txt"
    run cat "$SEEN/worktree"
    refute_output --partial "bad"
    cmp $'bad\xffname' "$TEST_TEMP_DIR/bad.orig"
    run git diff -z --name-only --diff-filter=A
    assert_output $'bad\xffname'
    run git diff --cached --name-only
    assert_output "staged.txt"
    run git stash list
    assert_output ""
}

@test "stash sets aside an intent-to-add file whose name is not UTF-8 with libgit2" {
    assert_non_utf8_intent_to_add_set_aside 1
}

@test "stash sets aside an intent-to-add file whose name is not UTF-8 with the git CLI" {
    assert_non_utf8_intent_to_add_set_aside 0
}

@test "files the stash skips keep their unstaged changes when intent-to-add files are set aside" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["append"] {
        glob = "*.sh"
        check = "true"
        check_list_files = "true"
        fix = "for f in {{files}}; do echo '# fixed' >> \\\$f; done"
      }
    }
  }
}
PKL
    printf 'echo hi\n' > a.sh
    git add hk.pkl a.sh
    git commit -m "init"
    # The worktree has the same contents as HEAD and only the index has the
    # new mode, so the stash sets nothing aside for a.sh, which still has an
    # unstaged change; the step must check it before fixing
    git update-index --chmod=+x a.sh
    printf 'new\n' > new.txt
    git add -N new.txt

    run hk run pre-commit
    assert_success
    run git ls-files -s a.sh
    assert_output --partial "100755"
    run cat a.sh
    assert_output "echo hi"
    run git status --porcelain
    assert_output "$(printf 'MM a.sh\n A new.txt')"
    run git stash list
    assert_output ""
}
