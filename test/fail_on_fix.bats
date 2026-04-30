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
