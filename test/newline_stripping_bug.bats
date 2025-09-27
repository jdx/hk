#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "fix preserves exact bytes when index has trailing newline" {
    # This test demonstrates the .read() stripping bug in origin/main
    # The bug: git_cmd().read() strips trailing newlines from git cat-file output

    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["formatter"] {
        glob = "test.txt"
        stage = "test.txt"
        // Formatter that adds a prefix
        fix = "sed 's/^/PREFIX:/' test.txt > test.tmp && mv test.tmp test.txt"
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install

    # Create base
    echo "base" > test.txt
    git add test.txt
    git -c commit.gpgsign=false commit -m "base"

    # Stage content with trailing newline
    printf 'staged\n' > test.txt
    git add test.txt

    # Worktree adds extra line
    printf 'staged\nworktree\n' > test.txt

    # Run fix with trace to see the bug
    run bash -c "HK_LOG_LEVEL=trace hk fix 2>&1 | grep 'manual-unstash.*ends_i=' | head -1"
    echo "Debug output: $output"

    # The bug: ends_i should be true (staged content ends with \n)
    # but .read() strips it, making ends_i=false
    if [[ "$output" == *"ends_i=false"* ]]; then
        echo "BUG DETECTED: Index newline was stripped (ends_i=false)"
        # This is the bug in origin/main
        [[ "$output" == *"ends_i=true"* ]]  # This will fail on origin/main
    else
        echo "FIX WORKING: Index newline preserved (ends_i=true)"
        assert_success
    fi
}
