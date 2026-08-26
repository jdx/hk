#!/usr/bin/env bats

# Tests for the default stage behavior: steps with fix commands
# automatically get stage="<JOB_FILES>" if not explicitly set,
# but only when hook-level staging is enabled.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "manual fix hook does not stage by default" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
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

    # Modify file to create unstaged changes
    echo "modified" > file.txt

    hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "hook stage=false prevents staging even with default step stage" {
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

    # Modify file to create unstaged changes
    echo "modified" > file.txt

    hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "HK_STAGE=0 prevents staging even with default step stage" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
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

    # Modify file to create unstaged changes
    echo "modified" > file.txt

    HK_STAGE=0 hk run fix

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "explicit step stage filters paths when hook staging is enabled" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = true
        steps {
            ["add-newline"] {
                glob = "*.txt"
                fix = #"for f in {{ files }}; do echo >> \$f; done"#
                stage = List("*.log")
            }
        }
    }
}
EOF
    echo -n "no newline" > file.txt
    echo "log content" > file.log
    git add hk.pkl file.txt file.log
    git commit -m "initial commit"

    echo "modified" > file.txt
    echo "modified log" > file.log

    hk run fix

    run git status --porcelain
    assert_success
    assert_output --partial 'M  file.log'
    assert_output --partial ' M file.txt'
}

@test "explicit step stage does not enable hook staging" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["add-newline"] {
                glob = "*.txt"
                fix = #"for f in {{ files }}; do echo >> \$f; done"#
                stage = List("*.log")
            }
        }
    }
}
EOF
    echo -n "no newline" > file.txt
    echo "log content" > file.log
    git add hk.pkl file.txt file.log
    git commit -m "initial commit"

    echo "modified" > file.txt
    echo "modified log" > file.log

    hk run fix

    run git status --porcelain
    assert_success
    assert_output --partial ' M file.log'
    assert_output --partial ' M file.txt'
}

@test "step without fix command doesn't auto-stage" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-only"] {
                glob = "*.txt"
                check = "cat {{ files }}"
            }
        }
    }
}
EOF
    echo "content" > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"

    # Modify file to create unstaged changes
    echo "modified" > file.txt

    hk run check

    # Check-only step should not stage anything
    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}

@test "--no-stage CLI flag prevents staging with default step stage" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
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

    # Modify file to create unstaged changes
    echo "modified" > file.txt

    hk run fix --no-stage

    run git status --porcelain
    assert_success
    assert_output ' M file.txt'
}
