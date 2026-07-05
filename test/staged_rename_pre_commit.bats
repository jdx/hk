#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

_setup_repo() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    steps {
      ["list"] {
        glob = "*.txt"
        check = "printf '%s\n' {{files}} > files_list.txt"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"

    seq 1 10 | sed 's/^/line /' > a.txt
    git add a.txt
    git commit -m "add a.txt"
}

_assert_step_ran_with_new_path() {
    run hk run pre-commit
    assert_success

    # Before the fix, the staged rename was invisible and the step was
    # silently skipped ("no files to run"), so files_list.txt never existed
    assert_file_exists files_list.txt
    assert_file_contains files_list.txt 'b\.txt'
    assert_file_not_contains files_list.txt 'a\.txt'
}

@test "pre-commit runs steps on staged pure rename (libgit2)" {
    export HK_LIBGIT2=1
    _setup_repo

    git mv a.txt b.txt

    # Sanity: ensure git detects the rename
    run bash -c "git status --porcelain"
    assert_output --partial "R  a.txt -> b.txt"

    _assert_step_ran_with_new_path
}

@test "pre-commit runs steps on staged pure rename (shell git)" {
    export HK_LIBGIT2=0
    _setup_repo

    git mv a.txt b.txt

    # Sanity: ensure git detects the rename
    run bash -c "git status --porcelain"
    assert_output --partial "R  a.txt -> b.txt"

    _assert_step_ran_with_new_path
}

@test "pre-commit runs steps on staged rename with modification (libgit2)" {
    export HK_LIBGIT2=1
    _setup_repo

    git mv a.txt b.txt
    echo "line 11" >> b.txt
    git add b.txt

    # Sanity: ensure git still detects this as a rename, not delete+add
    run bash -c "git status --porcelain"
    assert_output --partial "R  a.txt -> b.txt"

    _assert_step_ran_with_new_path
}

@test "pre-commit runs steps on staged rename with modification (shell git)" {
    export HK_LIBGIT2=0
    _setup_repo

    git mv a.txt b.txt
    echo "line 11" >> b.txt
    git add b.txt

    # Sanity: ensure git still detects this as a rename, not delete+add
    run bash -c "git status --porcelain"
    assert_output --partial "R  a.txt -> b.txt"

    _assert_step_ran_with_new_path
}
