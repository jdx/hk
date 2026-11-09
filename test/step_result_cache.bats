#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_CACHE=1
    export HK_CACHE_DIR="$TEST_TEMP_DIR/hk-cache"
}
teardown() {
    _common_teardown
}

# A `check` step that appends a line to runs.log each time it actually runs.
write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.txt"
                check = "echo run >> runs.log; ! grep -q bad {{files}}"
                // EXTRA
            }
        }
    }
}
EOF
    # Substitute the step's extra settings (kept out of the heredoc so it stays valid Pkl).
    EXTRA="$1" perl -0pi -e 's{// EXTRA}{$ENV{EXTRA}}' hk.pkl
}

runs() { wc -l < runs.log | tr -d ' '; }

@test "cache = true skips a check that already passed on the same contents" {
    write_config "cache = true"
    echo ok > a.txt
    git add hk.pkl a.txt
    hk check --all
    assert_equal "$(runs)" 1
    run hk check --all
    assert_success
    assert_equal "$(runs)" 1
}

@test "changed contents run the check again, a touched file does not" {
    write_config "cache = true"
    echo ok > a.txt
    git add hk.pkl a.txt
    hk check --all
    touch a.txt
    hk check --all
    assert_equal "$(runs)" 1
    echo changed > a.txt
    hk check --all
    assert_equal "$(runs)" 2
}

@test "a failed check is not cached" {
    write_config "cache = true"
    echo bad > a.txt
    git add hk.pkl a.txt
    run hk check --all
    assert_failure
    run hk check --all
    assert_failure
    assert_equal "$(runs)" 2
}

@test "a step without cache = true always runs" {
    write_config ""
    echo ok > a.txt
    git add hk.pkl a.txt
    hk check --all
    hk check --all
    assert_equal "$(runs)" 2
}

@test "HK_CACHE=0 turns the result cache off" {
    write_config "cache = true"
    echo ok > a.txt
    git add hk.pkl a.txt
    HK_CACHE=0 hk check --all
    HK_CACHE=0 hk check --all
    assert_equal "$(runs)" 2
}

@test "pre-commit skips a check that hk check passed on the same files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

local lint = new Step {
    glob = "*.txt"
    check = "echo run >> runs.log"
    cache = true
}

hooks {
    ["check"] { steps { ["lint"] = lint } }
    ["pre-commit"] { steps { ["lint"] = lint } }
}
EOF
    echo ok > a.txt
    git add hk.pkl a.txt
    hk check --all
    assert_equal "$(runs)" 1
    hk run pre-commit --all
    assert_equal "$(runs)" 1
}

@test "cache_inputs keys on the declared files, whatever the hook selected" {
    # The command does not take {{files}}, so it is the same whatever was selected.
    write_config 'cache = true
                cache_inputs = List("*.txt")'
    sed -i 's/ {{files}}/ *.txt/' hk.pkl
    echo ok > a.txt
    echo ok > b.txt
    git add hk.pkl a.txt b.txt
    hk check a.txt
    assert_equal "$(runs)" 1
    hk check b.txt
    assert_equal "$(runs)" 1
    echo changed > b.txt
    hk check a.txt
    assert_equal "$(runs)" 2
}

@test "fix always runs even when cache = true" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["fix"] {
        steps {
            ["fmt"] {
                glob = "*.txt"
                fix = "echo fix >> fix.log"
                cache = true
            }
        }
    }
}
EOF
    echo ok > a.txt
    git add hk.pkl a.txt
    hk fix --all
    hk fix --all
    assert_equal "$(wc -l < fix.log | tr -d ' ')" 2
}

@test "cache_inputs that match no files never cache" {
    write_config 'cache = true
                cache_inputs = List("*.nomatch")'
    echo ok > a.txt
    git add hk.pkl a.txt
    hk check --all
    hk check --all
    assert_equal "$(runs)" 2
}
