#!/usr/bin/env bats

# Restoring the stash used to start about six git processes for each unstaged
# file, even when no step touched it. A file no step changed only needs its
# stashed contents written back, which takes one.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_SUMMARY_TEXT=1
}

teardown() {
    _common_teardown
}

# Counts the git processes of the last trace that run subcommand $1.
count_git() {
    # GIT_TRACE=<file> appends one line per process; GIT_TRACE=1 would mix
    # the lines into hk's terminal output
    grep -cE "trace: (built-in|run_command|exec): git( -[^ ]+( [^- ][^ ]*)?)* $1( |\$)" "$TEST_TEMP_DIR/trace" || true
}

create_files() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "true" } }
  }
}
PKL
    printf 'base\n' > staged.txt
    for i in $(seq 1 "$1"); do
        printf 'base %s\n' "$i" > "file$i.txt"
    done
    git add .
    git commit -m init
    # The commit has one staged file; the others only have unstaged edits
    printf 'staged\n' > staged.txt
    git add staged.txt
    for i in $(seq 1 "$1"); do
        printf 'base %s\nunstaged %s\n' "$i" "$i" > "file$i.txt"
    done
}

check_restore_spawns() {
    local files=20
    create_files "$files"

    GIT_TRACE="$TEST_TEMP_DIR/trace" HK_LIBGIT2=$1 run hk run pre-commit
    assert_success

    for i in $(seq 1 "$files"); do
        assert_equal "$(cat "file$i.txt")" "$(printf 'base %s\nunstaged %s' "$i" "$i")"
    done
    assert_equal "$(git stash list)" ""

    # One read of each untouched file's stashed contents, and a few lookups
    # for the staged file, instead of several per file
    local ls_tree cat_file
    ls_tree=$(count_git ls-tree)
    cat_file=$(count_git cat-file)
    echo "ls-tree=$ls_tree cat-file=$cat_file"
    [ "$ls_tree" -le 4 ]
    [ "$cat_file" -le $((files + 5)) ]
}

@test "restoring untouched files starts one git process each (HK_LIBGIT2=1)" {
    check_restore_spawns 1
}

@test "restoring untouched files starts one git process each (HK_LIBGIT2=0)" {
    check_restore_spawns 0
}

# A required filter that fails must not turn the stashed contents into an
# empty or deleted file.
@test "a failing required filter keeps the stash and the untouched file" {
    git config filter.example.clean cat
    git config filter.example.smudge cat
    git config filter.example.required true
    printf 'crlf.txt filter=example text eol=crlf\n' > .gitattributes
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["break-filter"] { check = "git config filter.example.smudge false" }
    }
  }
}
PKL
    printf 'one\r\n' > crlf.txt
    printf 'one\n' > lf.txt
    git add .
    git commit -m init
    printf 'two\r\n' > crlf.txt
    printf 'two\n' > lf.txt
    git add .
    printf 'three\r\n' > crlf.txt
    printf 'three\n' > lf.txt

    run hk run pre-commit
    assert_failure

    test -f crlf.txt
    # The file is left as the stash left it, and the stash has its edit
    assert_equal "$(git show 'stash@{0}:crlf.txt')" "three"
    assert_equal "$(cat lf.txt)" "three"
}
