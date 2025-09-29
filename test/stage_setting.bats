#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "HK_STAGE=0 prevents automatic staging of fixed files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["generate"] {
                glob = "src/*.txt"
                stage = List("generated.txt")
                fix = "echo 'generated content' > generated.txt"
            }
        }
    }
}
EOF

    mkdir -p src
    echo "original content" > src/test.txt

    git add hk.pkl src/test.txt
    git commit -m "init"
    hk install

    # Make a change to trigger the hook
    echo "modified content" > src/test.txt
    git add src/test.txt

    # Run with HK_STAGE=0 to prevent auto-staging
    HK_STAGE=0 hk run pre-commit

    # Verify that src/test.txt is still staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "src/test.txt"

    # Verify that generated.txt is NOT staged
    refute_output --partial "generated.txt"

    # Verify that generated.txt exists but is untracked
    run git status --porcelain --untracked-files=all
    assert_success
    assert_output --partial "?? generated.txt"
}

@test "HK_STAGE=1 (default) stages fixed files automatically" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["generate"] {
                glob = "src/*.txt"
                stage = List("generated.txt")
                fix = "echo 'generated content' > generated.txt"
            }
        }
    }
}
EOF

    mkdir -p src
    echo "original content" > src/test.txt

    git add hk.pkl src/test.txt
    git commit -m "init"
    hk install

    # Make a change to trigger the hook
    echo "modified content" > src/test.txt
    git add src/test.txt

    # Run with default HK_STAGE (should be 1)
    hk run pre-commit

    # Verify that src/test.txt is staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "src/test.txt"

    # Verify that generated.txt IS staged
    assert_output --partial "generated.txt"

    # Verify that generated.txt is staged (A = added)
    run git status --porcelain
    assert_success
    assert_output --partial "A  generated.txt"
}

@test "git config hk.stage=false prevents automatic staging" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["generate"] {
                glob = "src/*.txt"
                stage = List("generated.txt")
                fix = "echo 'generated content' > generated.txt"
            }
        }
    }
}
EOF

    mkdir -p src
    echo "original content" > src/test.txt

    git add hk.pkl src/test.txt
    git commit -m "init"
    hk install

    # Set git config to disable staging
    git config hk.stage false

    # Make a change to trigger the hook
    echo "modified content" > src/test.txt
    git add src/test.txt

    # Run pre-commit
    hk run pre-commit

    # Verify that src/test.txt is still staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "src/test.txt"

    # Verify that generated.txt is NOT staged
    refute_output --partial "generated.txt"

    # Verify that generated.txt exists but is untracked
    run git status --porcelain --untracked-files=all
    assert_success
    assert_output --partial "?? generated.txt"
}
