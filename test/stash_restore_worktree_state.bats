#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_STASH_UNTRACKED=true
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "true" } }
  }
}
EOF
    echo base > f.txt
    echo base > d.txt
    echo a > a.txt
    git add -A
    git commit -qm init
}

teardown() {
    _common_teardown
}

# The index, and the whole worktree as a tree: contents, modes and symlinks.
snapshot() {
    git ls-files -s
    local index="$BATS_TEST_TMPDIR/snapshot-index"
    cp "$(git rev-parse --git-path index)" "$index"
    GIT_INDEX_FILE="$index" git add -A
    GIT_INDEX_FILE="$index" git write-tree
    rm -f "$index"
}

reset_repo() {
    git reset -q --hard
    git clean -qfd
}

@test "stash restores a file reverted to HEAD after a staged edit" {
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo staged >> f.txt
        git add f.txt
        echo base > f.txt
        # Another unstaged change, so that there is something to stash even
        # though f.txt matches HEAD
        echo unstaged >> d.txt
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        reset_repo
    done
}

@test "stash is kept when a step leaves a directory where it has a file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "mkdir -p u.txt && echo output > u.txt/output" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo untracked > u.txt

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_failure
        assert_output --partial "u.txt"
        assert_output --partial "Stash has been preserved"
        # The step's output is intact and the stash still has the file
        run cat u.txt/output
        assert_output output
        run git show 'stash@{0}^3:u.txt'
        assert_output untracked

        git stash drop -q
        rm -rf u.txt
        git reset -q --hard
    done
}

@test "stash is kept when a step writes a file where it has a symlink" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "rm -f link && echo output > link" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        ln -s a.txt link

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_failure
        assert_output --partial "a step changed link"
        assert_output --partial "Stash has been preserved"
        # The step's file is intact and the stash still has the symlink
        run test -L link
        assert_failure
        run cat link
        assert_output output
        run git cat-file -t 'stash@{0}^3:link'
        assert_output blob

        git stash drop -q
        rm -f link
        git reset -q --hard
    done
}

@test "stash restores an unstaged file replaced by an untracked directory" {
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        rm d.txt
        mkdir d.txt
        echo inner > d.txt/inner
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        rm -rf d.txt
        reset_repo
    done
}

@test "stash restore accepts a step repeating a stashed deletion" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "rm -f d.txt" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        rm d.txt
        echo untracked > w.txt
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        reset_repo
    done
}

@test "stash restores the other paths when one conflicts with a step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "echo step > u.txt" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo mine > u.txt
        echo other > w.txt
        rm d.txt

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_failure
        assert_output --partial "Did not restore u.txt from the stash"
        assert_output --partial "git restore --source='stash@{0}^3' -- ':(literal)u.txt'"
        # The step's file is kept, and every other stashed change is back
        run cat u.txt
        assert_output step
        run cat w.txt
        assert_output other
        assert_file_not_exists d.txt
        # The stash still has the conflicting file, which the hint restores
        git restore --source='stash@{0}^3' -- u.txt
        run cat u.txt
        assert_output mine

        git stash drop -q
        reset_repo
    done
}

@test "stash restores a file whose name starts with a double quote" {
    if ! touch '"probe' 2>/dev/null; then
        skip "filesystem rejects double quotes in names"
    fi
    rm -f '"probe'
    echo base > '"quoted.txt'
    git add '"quoted.txt'
    git commit -qm quoted
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo unstaged >> '"quoted.txt'
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        reset_repo
    done
}

@test "stash merges or keeps a step's change to a file it was not given" {
    printf 'line1\nline2\nline3\n' > d.txt
    git add d.txt
    git commit -qm lines
    for step in "{ echo step; cat d.txt; } > d.new && mv d.new d.txt" "sed -i.bak 's/line3/step3/' d.txt && rm d.txt.bak"; do
        cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { glob = "a.txt"; check = "$step" } }
  }
}
EOF
        git add hk.pkl
        git commit -qm "step $step"
        for libgit2 in 1 0; do
            echo staged >> a.txt
            git add a.txt
            sed -i.bak 's/line3/mine3/' d.txt && rm d.txt.bak

            HK_LIBGIT2=$libgit2 run hk run pre-commit
            case "$step" in
            *"echo step"*)
                # Different lines: both changes are kept
                assert_success
                run cat d.txt
                assert_output $'step\nline1\nline2\nmine3'
                assert_equal "$(git stash list)" ""
                ;;
            *)
                # The same line: the step's version stays and the stash is kept
                assert_failure
                assert_output --partial "a step changed d.txt, which the stash also changed, in the same lines"
                run cat d.txt
                assert_output $'line1\nline2\nstep3'
                git restore --source='stash@{0}' -- d.txt
                run cat d.txt
                assert_output $'line1\nline2\nmine3'
                git stash drop -q
                ;;
            esac
            reset_repo
        done
    done
}

@test "stash recovery advice lists every path it did not restore" {
    if [ "$(id -u)" = 0 ]; then
        skip "root can write to read-only directories"
    fi
    export NO_COLOR=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { check = "echo step > \"O'Brien.txt\" && chmod a-w sub" } }
  }
}
EOF
    mkdir sub
    echo tracked > sub/tracked.txt
    git add hk.pkl sub
    git commit -qm config
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        echo mine > "O'Brien.txt"
        echo untracked > sub/untracked.txt

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        chmod u+w sub
        assert_failure
        assert_output --partial "Did not restore O'Brien.txt, sub/untracked.txt from the stash"
        assert_output --partial "only once every path above is recovered"
        # Every printed command works as it is, quotes included
        commands=$(printf '%s\n' "$output" | sed -n 's/.*To take its stashed version, run: //p')
        assert_equal "$(printf '%s\n' "$commands" | wc -l | tr -d ' ')" 2
        eval "$commands"
        run cat "O'Brien.txt"
        assert_output mine
        run cat sub/untracked.txt
        assert_output untracked

        git stash drop -q
        reset_repo
    done
}

@test "stash restore keeps a step's mode change, with and without core.fileMode" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["probe"] { glob = "a.txt"; check = "chmod +x f.txt" } }
  }
}
EOF
    git add hk.pkl
    git commit -qm config
    for file_mode in true false; do
        git config core.fileMode $file_mode
        for libgit2 in 1 0; do
            echo staged >> a.txt
            git add a.txt
            echo staged >> f.txt
            git add f.txt
            echo unstaged >> f.txt

            HK_LIBGIT2=$libgit2 run hk run pre-commit
            assert_success
            run cat f.txt
            assert_output $'base\nstaged\nunstaged'
            # hk leaves the executable bit as the step set it
            run test -x f.txt
            assert_success
            assert_equal "$(git stash list)" ""
            chmod -x f.txt
            reset_repo
        done
    done
}

@test "stash is kept when a fixer with stage = false leaves output it cannot merge" {
    printf '\000\377binary' > binary.dat
    git add binary.dat
    git commit -qm binary
    for fix in "rm f.txt" "rm f.txt && ln -s a.txt f.txt" "cp binary.dat f.txt"; do
        cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stage = false
    stash = "git"
    steps { ["fixer"] { glob = "f.txt"; fix = "$fix" } }
  }
}
EOF
        git add hk.pkl
        git commit -qm "fix $fix"
        for libgit2 in 1 0; do
            echo staged >> f.txt
            git add f.txt
            echo unstaged >> f.txt
            HK_LIBGIT2=$libgit2 run hk run pre-commit
            assert_failure
            assert_output --partial "Did not restore f.txt from the stash"
            case "$fix" in
            "rm f.txt")
                assert_output --partial "a step changed f.txt, which the stash also changed"
                assert_file_not_exists f.txt
                ;;
            *ln*)
                assert_output --partial "a step changed f.txt, which the stash also changed"
                run readlink f.txt
                assert_output a.txt
                ;;
            *)
                assert_output --partial "a step wrote binary content to f.txt"
                refute_output --partial "restoring f.txt failed"
                run cmp f.txt binary.dat
                assert_success
                ;;
            esac
            # The stash still has the unmerged edits
            rm -f f.txt
            git restore --source='stash@{0}' -- f.txt
            run cat f.txt
            assert_output $'base\nstaged\nunstaged'
            git stash drop -q
            reset_repo
        done
    done
}

@test "a stash of untracked files restores unstaged deletions" {
    for libgit2 in 1 0; do
        echo staged >> a.txt
        git add a.txt
        rm d.txt
        echo untracked > untracked.txt
        before=$(snapshot)

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        assert_success
        assert_equal "$(snapshot)" "$before"
        assert_equal "$(git stash list)" ""
        reset_repo
    done
}

# With nothing else unstaged, the stash is limited to the files a step runs on.
# A file reverted to HEAD has no HEAD-to-worktree diff for that, so steps used
# to run on the reverted contents and stage them over the staged edit.
@test "a fix step keeps a staged edit when the worktree copy was reverted to HEAD" {
    unset HK_STASH_UNTRACKED
    export HK_STATE_DIR="$TEST_TEMP_DIR/state"
    # sed -i differs between GNU and BSD, so the changing fixer writes a copy
    for fixer in "true" "sed s/l1/L1/ f.txt > f.tmp && mv f.tmp f.txt"; do
        cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps { ["fixer"] { glob = "f.txt"; fix = "$fixer" } }
  }
}
EOF
        git add hk.pkl
        printf 'l1\nl2\nl3\n' > f.txt
        git add f.txt
        git commit -qm "base $fixer"
        for libgit2 in 1 0; do
            printf 'l1\nl2\nl3\nstaged\n' > f.txt
            git add f.txt
            printf 'l1\nl2\nl3\n' > f.txt
            rm -rf "$HK_STATE_DIR"

            HK_LIBGIT2=$libgit2 run hk run pre-commit
            assert_success
            if [ "$fixer" = true ]; then
                index=$'l1\nl2\nl3\nstaged'
                worktree=$'l1\nl2\nl3'
            else
                # The fixer ran on the staged contents, and the reverted
                # worktree keeps its own state plus the fixer's change
                index=$'L1\nl2\nl3\nstaged'
                worktree=$'L1\nl2\nl3'
            fi
            assert_equal "$(git show :f.txt)" "$index"
            assert_equal "$(cat f.txt)" "$worktree"
            assert_equal "$(git stash list)" ""
            # The backup patch has the reverted edit, which `git stash show`
            # would not, since the stash matches HEAD
            run cat "$HK_STATE_DIR"/patches/*.patch
            assert_output --partial "-staged"
            reset_repo
        done
    done
}

@test "reverted files are restored when checking out the staged contents fails midway" {
    unset HK_STASH_UNTRACKED
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps { ["fixer"] { glob = "**/*.txt"; fix = "true" } }
  }
}
EOF
    mkdir sub
    printf 'one\n' > a.txt
    printf 'one\n' > sub/b.txt
    git add .
    git commit -qm "files"
    for libgit2 in 1 0; do
        printf 'one\nstaged\n' > a.txt
        printf 'one\nstaged\n' > sub/b.txt
        git add a.txt sub/b.txt
        printf 'one\n' > a.txt
        printf 'one\n' > sub/b.txt
        # a.txt is checked out first; then sub/b.txt cannot be replaced
        chmod 555 sub

        HK_LIBGIT2=$libgit2 run hk run pre-commit
        chmod 755 sub
        assert_failure

        # The files the checkout reached are back as the user left them
        assert_equal "$(cat a.txt)" one
        assert_equal "$(cat sub/b.txt)" one
        assert_equal "$(git show :a.txt)" "$(printf 'one\nstaged')"
        assert_equal "$(git stash list)" ""
        reset_repo
    done
}
