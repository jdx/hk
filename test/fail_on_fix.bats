#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "fail_on_fix=true fails when fixer modifies files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = false
        fail_on_fix = true
        steps {
            ["add-newline"] {
                glob = "*.txt"
                fix = #"for f in {{ files }}; do echo >> \$f; done"#
            }
        }
    }
}
EOF
    echo -n "no newline" > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"

    echo "modified" > file.txt

    run hk run fix
    assert_failure
}

@test "fail_on_fix=true passes when fixer does not modify files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = false
        fail_on_fix = true
        steps {
            ["noop"] {
                glob = "*.txt"
                fix = "true"
            }
        }
    }
}
EOF
    echo "content" > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"

    # Create an unstaged change so hk picks up the file
    echo "modified" > file.txt

    # Fixer is a no-op (true), so file content stays the same and fail_on_fix should not trigger
    hk run fix
}

@test "fail_on_fix=true ignores pre-existing unstaged files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = false
        fail_on_fix = true
        steps {
            ["noop"] {
                glob = "*.txt"
                fix = "true"
            }
        }
    }
}
EOF
    echo "content" > file.txt
    echo "other" > other.txt
    git add hk.pkl file.txt other.txt
    git commit -m "initial commit"

    # Create pre-existing unstaged change in an unrelated file
    echo "changed" > other.txt

    # Fixer is a no-op, so fail_on_fix should NOT trigger despite unstaged other.txt
    hk run fix
}

@test "fail_on_fix=false (default) passes when fixer modifies files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = false
        steps {
            ["add-newline"] {
                glob = "*.txt"
                fix = #"for f in {{ files }}; do echo >> \$f; done"#
            }
        }
    }
}
EOF
    echo -n "no newline" > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"

    echo "modified" > file.txt

    hk run fix
}

@test "fail_on_fix=true preserves staged changes and surfaces fix as unstaged (#888)" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        fail_on_fix = true
        steps {
            ["normalize"] {
                glob = "*.json"
                fix = #"for f in {{ files }}; do tr -d ' ' < "\$f" > "\$f.tmp" && mv "\$f.tmp" "\$f"; done"#
            }
        }
    }
}
EOF
    # Initial committed state has spaces that the fixer will strip.
    echo '{"a": 1}' > a.json
    echo "original" > b.md
    git add hk.pkl a.json b.md
    git commit -m "initial commit"
    hk install

    # User makes intentional changes to both files, but only stages a.json.
    echo '{"a": 2}' > a.json
    echo "modified" > b.md
    git add a.json

    # Pre-commit must fail with fail_on_fix.
    run git commit -m "update"
    assert_failure

    # The user's staged change to a.json must survive: index still differs from HEAD
    # in the test value, NOT in the formatting (which is the fixer's contribution).
    run git diff --cached --name-only
    assert_output "a.json"
    run git diff --cached a.json
    assert_output --partial '"a": 2'
    refute_output --partial '{"a":2}'

    # The fix should now be visible as an unstaged change on a.json (whitespace removed).
    run git diff --name-only
    assert_line "a.json"
    assert_line "b.md"

    # b.md unstaged change must be preserved.
    run cat b.md
    assert_output "modified"
}

@test "fail_on_fix=true preserves fixer output in a partially staged file (#1144)" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        stage = false
        fail_on_fix = true
        steps {
            ["format"] {
                glob = "*.js"
                fix = #"for f in {{ files }}; do sed 's/x=2/x = 2/' "\$f" > "\$f.tmp" && mv "\$f.tmp" "\$f"; done"#
            }
        }
    }
}
EOF
    cat <<'EOF' > partial.js
const x = 1;
const y = 1;
const z = 1;
EOF
    git add hk.pkl partial.js
    git commit -m "initial commit"

    sed 's/x = 1/x=2/' partial.js > partial.js.tmp
    mv partial.js.tmp partial.js
    git add partial.js
    sed 's/z = 1/z=2/' partial.js > partial.js.tmp
    mv partial.js.tmp partial.js

    run hk run pre-commit
    assert_failure

    # The index retains only the user's staged change, without the fixer's formatting.
    run git show :partial.js
    assert_line "const x=2;"
    assert_line "const z = 1;"

    # The worktree combines the formatter output with the user's unstaged change.
    run cat partial.js
    assert_line "const x = 2;"
    assert_line "const z=2;"
}

@test "fail_on_fix=true preserves fixer deletion of a partially staged file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        stage = false
        fail_on_fix = true
        steps {
            ["delete"] {
                glob = "deleted.txt"
                fix = "rm -- {{ files }}"
            }
        }
    }
}
EOF
    cat <<'EOF' > deleted.txt
x = 1
y = 1
z = 1
EOF
    git add hk.pkl deleted.txt
    git commit -m "initial commit"

    sed 's/x = 1/x=2/' deleted.txt > deleted.txt.tmp
    mv deleted.txt.tmp deleted.txt
    git add deleted.txt
    sed 's/z = 1/z=2/' deleted.txt > deleted.txt.tmp
    mv deleted.txt.tmp deleted.txt

    run hk run pre-commit
    assert_failure

    # The deletion remains in the worktree while the original staged blob stays in the index.
    run test -e deleted.txt
    assert_failure
    run git show :deleted.txt
    assert_line "x=2"
    assert_line "z = 1"
}

@test "fail_on_fix=true preserves binary fixer output for a partially staged file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        stage = false
        fail_on_fix = true
        steps {
            ["binary"] {
                glob = "binary.dat"
                fix = #"for f in {{ files }}; do printf '\377' > "\$f"; done"#
            }
        }
    }
}
EOF
    cat <<'EOF' > binary.dat
x = 1
y = 1
z = 1
EOF
    git add hk.pkl binary.dat
    git commit -m "initial commit"

    sed 's/x = 1/x=2/' binary.dat > binary.dat.tmp
    mv binary.dat.tmp binary.dat
    git add binary.dat
    sed 's/z = 1/z=2/' binary.dat > binary.dat.tmp
    mv binary.dat.tmp binary.dat

    run hk run pre-commit
    assert_failure

    # The binary fixer result remains in the worktree while the index stays unchanged.
    run od -An -tx1 binary.dat
    assert_output --partial "ff"
    run git show :binary.dat
    assert_line "x=2"
    assert_line "z = 1"
}
