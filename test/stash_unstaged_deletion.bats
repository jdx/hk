#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

_write_config() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    stage = false
    fix = false
    steps {
      ["assert-index-file-present"] {
        check = "test -f tracked.txt && grep -qx baseline extra.txt"
      }
    }
  }
}
PKL
    for f in tracked.txt other.txt extra.txt; do echo baseline > "$f"; done
    git add .
    git commit -m "initial commit"
    echo "staged edit" > other.txt
    git add other.txt
}

@test "pre-commit: unstaged deletion is hidden from the hook and kept afterwards" {
    _write_config
    rm tracked.txt
    index_before=$(git ls-files --stage)

    run hk run pre-commit --check
    assert_success

    assert_file_not_exists tracked.txt
    assert_equal "$(git ls-files --stage)" "$index_before"
    run git status --porcelain
    assert_output --partial " D tracked.txt"
    run git for-each-ref refs/stash refs/hk/stash
    assert_output ""
}

@test "pre-commit: unstaged deletion is hidden from the hook alongside an unstaged edit" {
    _write_config
    rm tracked.txt
    echo "unstaged edit" > extra.txt

    run hk run pre-commit --check
    assert_success

    assert_file_not_exists tracked.txt
    assert_equal "$(cat extra.txt)" "unstaged edit"
    run git status --porcelain
    assert_output --partial " D tracked.txt"
    assert_output --partial " M extra.txt"
    assert_output --partial "M  other.txt"
}

@test "pre-commit: a failing hook still keeps the unstaged deletion" {
    _write_config
    sed -i.bak 's|check = "\(.*\)"|check = "\1 \&\& touch .git/seen-tracked; false"|' hk.pkl
    rm hk.pkl.bak
    rm tracked.txt

    run hk run pre-commit --check
    assert_failure

    # The failing check saw the indexed file
    assert_file_exists .git/seen-tracked
    assert_file_not_exists tracked.txt
    run git status --porcelain
    assert_output --partial " D tracked.txt"
    assert_output --partial "M  other.txt"
}

@test "pre-commit: a step that stages the deletion leaves no checkout behind" {
    _write_config
    sed -i.bak 's|check = "\(.*\)"|check = "\1 \&\& git rm -q --cached tracked.txt"|' hk.pkl
    rm hk.pkl.bak
    rm tracked.txt

    run hk run pre-commit --check
    assert_success

    assert_file_not_exists tracked.txt
    run git status --porcelain
    assert_output --partial "D  tracked.txt"
    refute_output --partial "?? tracked.txt"
}

@test "pre-commit: a step that stages the deletion and writes the file keeps its output" {
    _write_config
    sed -i.bak 's|check = "\(.*\)"|check = "\1 \&\& git rm -q --cached tracked.txt \&\& echo step-output > tracked.txt"|' hk.pkl
    rm hk.pkl.bak
    rm tracked.txt

    run hk run pre-commit --check
    assert_failure

    assert_equal "$(cat tracked.txt)" "step-output"
    # hk keeps the stash entry because it did not restore everything
    run git for-each-ref refs/hk/stash
    assert_output --partial "refs/hk/stash/"
}

@test "pre-commit: a step that stages the deletion of a symlink leaves no checkout behind" {
    _write_config
    ln -s other.txt link.txt
    git add link.txt
    git commit -m "add symlink"
    sed -i.bak 's|check = "\(.*\)"|check = "\1 \&\& test -L link.txt \&\& git rm -q --cached link.txt"|' hk.pkl
    rm hk.pkl.bak
    rm link.txt

    run hk run pre-commit --check
    assert_success

    assert [ ! -e link.txt ] && [ ! -L link.txt ]
    run git status --porcelain
    assert_output --partial "D  link.txt"
    refute_output --partial "?? link.txt"
}

@test "pre-commit: a step that replaces a deleted file with a symlink keeps the symlink" {
    _write_config
    printf 'other.txt' > tracked.txt
    git add tracked.txt
    git commit -m "tracked file holds a link target"
    sed -i.bak 's|check = "\(.*\)"|check = "\1 \&\& git rm -q --cached tracked.txt \&\& rm tracked.txt \&\& ln -s other.txt tracked.txt"|' hk.pkl
    rm hk.pkl.bak
    rm tracked.txt

    run hk run pre-commit --check
    assert_failure

    assert_link_exists tracked.txt
    run git for-each-ref refs/hk/stash
    assert_output --partial "refs/hk/stash/"
}

@test "pre-commit: a step that replaces a deleted symlink with a regular file keeps the file" {
    _write_config
    ln -s other.txt link.txt
    git add link.txt
    git commit -m "add symlink"
    sed -i.bak 's|check = "\(.*\)"|check = "\1 \&\& git rm -q --cached link.txt \&\& rm link.txt \&\& printf other.txt > link.txt"|' hk.pkl
    rm hk.pkl.bak
    rm link.txt

    run hk run pre-commit --check
    assert_failure

    assert_file_exists link.txt
    assert [ ! -L link.txt ]
}
