#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "step glob relative to dir does not match outside dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ts-check"] {
                dir = "web"
                glob = List("js/**/*.ts")
                check = "echo files {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p web/js/components
    echo "console.log('in web');" > web/js/components/a.ts

    mkdir -p js
    echo "console.log('outside web');" > js/outside.ts

    git add web/js/components/a.ts js/outside.ts

    run hk check
    assert_success

    # Should only include the file under web/, rendered relative to dir (so no web/ prefix)
    assert_output --partial "files js/components/a.ts"

    # Must not include files outside the dir, even if they match the unprefixed glob
    refute_output --partial "js/outside.ts"
}

@test "exclude patterns are relative to dir and do not require dir prefix" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ts-check"] {
                dir = "web"
                glob = List("**/*.ts")
                exclude = List("js/**/ignore.ts")
                check = "echo files {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p web/js/components
    echo "console.log('in web');" > web/js/components/a.ts
    echo "console.log('ignore');" > web/js/components/ignore.ts

    mkdir -p js
    echo "console.log('outside web');" > js/outside.ts

    git add web/js/components/a.ts web/js/components/ignore.ts js/outside.ts

    run hk check
    assert_success

    # Should include only a.ts inside web, excluding ignore.ts via dir-relative exclude
    assert_output --partial "files js/components/a.ts"
    refute_output --partial "js/components/ignore.ts"
    refute_output --partial "js/outside.ts"
}

@test "stage patterns are relative to dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["fix-ts"] {
                dir = "web"
                glob = List("**/*.ts")
                fix = "sh -c 'echo //fixed >> js/components/a.ts'"
                stage = "js/components/a.ts"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p web/js/components
    echo "console.log('in web');" > web/js/components/a.ts
    git add web/js/components/a.ts

    run hk run pre-commit
    assert_success

    # Ensure the file got staged (relative stage path applied under dir)
    run git diff --name-only --cached
    assert_success
    assert_output --partial "web/js/components/a.ts"
}
