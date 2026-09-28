#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    git config core.autocrlf false
    cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["noop"] { check = "true" }
    }
  }
}
PKL
}

teardown() {
    _common_teardown
}

check_untouched_file() {
    printf 'one\r\n' > crlf.txt
    printf 'one\n' > lf.txt
    git add .
    git commit -m init
    printf 'two\r\n' > crlf.txt
    printf 'two\n' > lf.txt
    git add lf.txt
    cp crlf.txt "$TEST_TEMP_DIR/expected"
    git diff --cached --binary > "$TEST_TEMP_DIR/index"

    hk run pre-commit

    cmp crlf.txt "$TEST_TEMP_DIR/expected"
    git diff --cached --binary > "$TEST_TEMP_DIR/index-after"
    cmp "$TEST_TEMP_DIR/index" "$TEST_TEMP_DIR/index-after"
    assert_equal "$(git stash list)" ""
}

@test "git stash restores untouched CRLF file using gitattributes" {
    printf 'crlf.txt text eol=crlf\nlf.txt text eol=lf\n' > .gitattributes
    check_untouched_file
}

@test "git stash restores untouched CRLF file using core.autocrlf" {
    git config core.autocrlf true
    printf 'lf.txt text eol=lf\n' > .gitattributes
    check_untouched_file
}

@test "git stash restores large CRLF files and binary files" {
    printf 'large.txt text eol=crlf\nbinary.dat -text\n' > .gitattributes
    printf 'one\n' > staged.txt
    printf 'one\r\n' > large.txt
    printf '\000\377\r\n' > binary.dat
    git add .
    git commit -m init
    awk 'BEGIN { for (i=0; i<100001; i++) printf "0123456789\r\n" }' > large.txt
    printf '\000\376\r\n' > binary.dat
    printf 'two\n' > staged.txt
    git add staged.txt
    cp large.txt "$TEST_TEMP_DIR/large"
    cp binary.dat "$TEST_TEMP_DIR/binary"

    hk run pre-commit

    cmp large.txt "$TEST_TEMP_DIR/large"
    cmp binary.dat "$TEST_TEMP_DIR/binary"
    assert_equal "$(git stash list)" ""
}

@test "git stash merges fixer and unstaged edits in CRLF worktree form" {
    cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["fixer"] {
        glob = "crlf.txt"
        fix = "printf 'fixed\\r\\nunchanged\\r\\nlast\\r\\n' > crlf.txt"
      }
    }
  }
}
PKL
    printf 'crlf.txt text eol=crlf\n' > .gitattributes
    printf 'first\r\nunchanged\r\nlast\r\n' > crlf.txt
    git add .
    git commit -m init
    printf 'staged\r\nunchanged\r\nlast\r\n' > crlf.txt
    git add crlf.txt
    printf 'staged\r\nunchanged\r\nunstaged\r\n' > crlf.txt

    hk run pre-commit

    printf 'fixed\r\nunchanged\r\nunstaged\r\n' > "$TEST_TEMP_DIR/expected"
    cmp crlf.txt "$TEST_TEMP_DIR/expected"
    git show :crlf.txt > "$TEST_TEMP_DIR/index"
    printf 'fixed\nunchanged\nlast\n' > "$TEST_TEMP_DIR/expected-index"
    cmp "$TEST_TEMP_DIR/index" "$TEST_TEMP_DIR/expected-index"
    assert_equal "$(git stash list)" ""
}

@test "git stash retains recovery data when a required checkout filter fails" {
    git config filter.example.clean cat
    git config filter.example.smudge cat
    git config filter.example.required true
    printf 'crlf.txt filter=example text eol=crlf\nlf.txt text eol=lf\n' > .gitattributes
    cat > hk.pkl <<PKL
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
    git add lf.txt

    run hk run pre-commit
    assert_failure

    # Failed conversion must leave the checkout intact and retain the stash.
    printf 'one\r\n' > "$TEST_TEMP_DIR/expected"
    cmp crlf.txt "$TEST_TEMP_DIR/expected"
    git show 'stash@{0}:crlf.txt' > "$TEST_TEMP_DIR/stashed"
    printf 'two\n' > "$TEST_TEMP_DIR/expected-stashed"
    cmp "$TEST_TEMP_DIR/stashed" "$TEST_TEMP_DIR/expected-stashed"
}

check_failed_merge_input() {
    local rejected="$1"
    git config filter.example.clean cat
    git config filter.example.smudge cat
    git config filter.example.required true
    printf 'crlf.txt filter=example text eol=crlf\n' > .gitattributes
    cat > filter.sh <<SH
#!/bin/sh
contents=\$(cat)
case "\$contents" in $rejected*) exit 1 ;; esac
printf '%s\n' "\$contents"
SH
    cat > fix.sh <<'SH'
#!/bin/sh
# Stage output without changing the isolated worktree, exercising the index read.
oid=$(printf 'fixed\n' | git hash-object -w --stdin)
git update-index --cacheinfo "100644,$oid,crlf.txt"
git config filter.example.smudge 'sh filter.sh'
SH
    cat > hk.pkl <<PKL
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["fixer"] {
        glob = "crlf.txt"
        check = "sh fix.sh"
      }
    }
  }
}
PKL
    printf 'base\r\n' > crlf.txt
    git add .
    git commit -m init
    printf 'staged\r\n' > crlf.txt
    git add crlf.txt
    printf 'unstaged\r\n' > crlf.txt

    run hk run pre-commit
    assert_failure
    assert_output --partial 'failed to read merge inputs'

    printf 'staged\r\n' > "$TEST_TEMP_DIR/expected"
    cmp crlf.txt "$TEST_TEMP_DIR/expected"
    assert_equal "$(git show :crlf.txt)" fixed
    assert_equal "$(git show 'stash@{0}:crlf.txt')" unstaged
}

@test "git stash retains worktree and stash when the base filter fails" {
    check_failed_merge_input base
}

@test "git stash retains worktree and stash when the index filter fails" {
    check_failed_merge_input staged
}

@test "git stash retains worktree and stash when the fixer filter fails" {
    check_failed_merge_input fixed
}

@test "git stash restores text replacing binary history" {
    printf '\000\377\n' > history.dat
    printf 'one\n' > lf.txt
    git add .
    git commit -m init
    # Cover a binary index as well as a text index with binary HEAD history.
    for index_kind in binary text; do
        if [ "$index_kind" = text ]; then
            printf 'staged text\n' > history.dat
            git add history.dat
        fi
        printf 'unstaged text\n' > history.dat
        printf 'two\n' > lf.txt
        git add lf.txt
        git write-tree > "$TEST_TEMP_DIR/index"

        hk run pre-commit

        assert_equal "$(cat history.dat)" 'unstaged text'
        git write-tree > "$TEST_TEMP_DIR/index-after"
        cmp "$TEST_TEMP_DIR/index" "$TEST_TEMP_DIR/index-after"
        assert_equal "$(git stash list)" ""
    done
}
