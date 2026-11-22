setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# `lint` passes unless a file contains BAD; `fmt` runs after it and appends
# `$1` to each file.
_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
local linters = new Mapping<String, Step> {
    ["lint"] {
        glob = "*.txt"
        check = "echo ran >> '$BATS_TEST_TMPDIR/lint-runs'; ! grep -l BAD {{files}}"
    }
    ["fmt"] {
        glob = "*.txt"
        depends = "lint"
        fix = "for f in {{files}}; do printf '$1' >> \"\$f\"; done"
    }
}
hooks {
    ["check"] { steps = linters }
    ["fix"] { fix = true; steps = linters }
    ["pre-commit"] { fix = true; steps = linters }
}
EOF
    git add hk.pkl
    git commit -qm "initial commit"
}

_lint_runs() {
    wc -l < "$BATS_TEST_TMPDIR/lint-runs" | tr -d ' '
}

@test "a check that passed runs again when a later fix changes its files, and fails" {
    _config 'BAD\\n'
    echo ok > a.txt
    git add a.txt

    run hk fix
    assert_failure
    assert_output --partial "a.txt"
    assert_equal "$(_lint_runs)" 2
}

@test "the commit is blocked when a fix breaks a check that already passed" {
    _config 'BAD\\n'
    hk install
    echo ok > a.txt
    git add a.txt

    run git commit -m "add a"
    assert_failure
    assert_equal "$(_lint_runs)" 2
    run git log --format=%s -1
    assert_output "initial commit"
}

@test "the check runs again and passes when the fix keeps its files passing" {
    _config 'fine\\n'
    echo ok > a.txt
    git add a.txt

    hk fix
    assert_equal "$(_lint_runs)" 2
    assert_equal "$(cat a.txt)" "$(printf 'ok\nfine')"
}

@test "a fix that leaves the files unchanged does not run the check again" {
    _config ''
    echo ok > a.txt
    git add a.txt

    hk fix
    assert_equal "$(_lint_runs)" 1
}

@test "hk check does not run checks twice" {
    _config 'BAD\\n'
    echo ok > a.txt
    git add a.txt

    hk check
    assert_equal "$(_lint_runs)" 1
}

@test "a check runs again after the checks it depends on run again" {
    # Read-effect checks share their file locks, so only the scheduling keeps
    # their second runs apart.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["slow"] {
                glob = "*.txt"
                check = new CommandSpec { command = "sleep 0.5; echo slow >> '$BATS_TEST_TMPDIR/log'"; effect = "read" }
            }
            ["after_slow"] {
                glob = "*.txt"
                depends = "slow"
                check = new CommandSpec { command = "echo after_slow >> '$BATS_TEST_TMPDIR/log'"; effect = "read" }
            }
            ["fmt"] {
                glob = "*.txt"
                depends = List("slow", "after_slow")
                fix = "for f in {{files}}; do echo fixed >> \"\$f\"; done"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -qm "initial commit"
    echo ok > a.txt
    git add a.txt

    hk fix
    assert_equal "$(cat "$BATS_TEST_TMPDIR/log")" "$(printf 'slow\nafter_slow\nslow\nafter_slow')"
}

@test "exclusive checks run again one at a time" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["one"] {
                glob = "*.txt"
                exclusive = true
                check = new CommandSpec { command = "echo one start >> '$BATS_TEST_TMPDIR/log'; sleep 0.5; echo one end >> '$BATS_TEST_TMPDIR/log'"; effect = "read" }
            }
            ["two"] {
                glob = "*.txt"
                exclusive = true
                check = new CommandSpec { command = "echo two start >> '$BATS_TEST_TMPDIR/log'; sleep 0.5; echo two end >> '$BATS_TEST_TMPDIR/log'"; effect = "read" }
            }
            ["fmt"] {
                glob = "*.txt"
                fix = "for f in {{files}}; do echo fixed >> \"\$f\"; done"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -qm "initial commit"
    echo ok > a.txt
    git add a.txt

    hk fix
    local once
    once="$(printf 'one start\none end\ntwo start\ntwo end')"
    assert_equal "$(cat "$BATS_TEST_TMPDIR/log")" "$once"$'\n'"$once"
}

@test "a check runs again when a patch changes its file outside the patching job" {
    cat <<'SCRIPT' > patch-b.sh
#!/usr/bin/env bash
printf -- '--- a/b.txt\n+++ b/b.txt\n@@ -1 +1,2 @@\n ok\n+BAD\n'
exit 1
SCRIPT
    chmod +x patch-b.sh
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["lint"] {
                glob = "b.txt"
                check = "echo ran >> '$BATS_TEST_TMPDIR/lint-runs'; ! grep -l BAD {{files}}"
            }
            ["patcher"] {
                glob = "a.txt"
                depends = "lint"
                check_diff = "./patch-b.sh"
                fix = "true"
            }
        }
    }
}
EOF
    git add hk.pkl patch-b.sh
    git commit -qm "initial commit"
    echo ok > a.txt
    echo ok > b.txt
    git add a.txt b.txt

    run hk fix
    assert_failure
    assert_equal "$(cat b.txt)" "$(printf 'ok\nBAD')"
    assert_equal "$(_lint_runs)" 2
}
