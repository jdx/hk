#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk fix --from-ref and --to-ref fixes files between refs" {
    # Create a file and commit it
    cat <<EOF > test1.js
console.log("test1")
EOF
    git add test1.js
    git commit -m "Add test1.js"

    # Save the first commit hash
    FIRST_COMMIT=$(git rev-parse HEAD)

    # Modify the file and commit it
    cat <<EOF > test1.js
console.log("test1 modified")
EOF
    git add test1.js
    git commit -m "Modify test1.js"

    # Create a new file and commit it
    cat <<EOF > test2.js
console.log("test2")
EOF
    git add test2.js
    git commit -m "Add test2.js"

    # Save the last commit hash
    LAST_COMMIT=$(git rev-parse HEAD)

    # Create the hk.pkl file with prettier
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        fix = true
        stash = "patch-file"
        steps {
            ["prettier"] = Builtins.prettier
        }
    }
}
EOF

    hk fix --from-ref=$FIRST_COMMIT --to-ref=$LAST_COMMIT

    # Verify files were formatted
    run cat test1.js
    assert_output 'console.log("test1 modified");'
    run cat test2.js
    assert_output 'console.log("test2");'

    # Create a third file but don't commit it
    cat <<EOF > test3.js
console.log("test3")
EOF

    # Run hk fix with --from-ref and --to-ref again
    hk fix --from-ref=$FIRST_COMMIT --to-ref=$LAST_COMMIT

    # Verify test3.js was not formatted
    run cat test3.js
    assert_output 'console.log("test3")'
}

@test "steps see the ref range as templates and HK_FROM_REF/HK_TO_REF" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["refs"] {
                check = "echo env=[\$HK_FROM_REF..\$HK_TO_REF] tpl=[{{from_ref}}..{{to_ref}}]"
            }
            ["override"] {
                env { ["HK_FROM_REF"] = "custom" }
                check = "echo override=\$HK_FROM_REF"
            }
        }
    }
}
EOF
    git init
    git add hk.pkl
    git commit -m "initial commit"
    echo a > a.txt
    git add a.txt
    git commit -m "second commit"

    run hk check --from-ref HEAD~1 --to-ref HEAD
    assert_success
    assert_output --partial "env=[HEAD~1..HEAD] tpl=[HEAD~1..HEAD]"
    assert_output --partial "override=custom"

    run hk check --all
    assert_success
    assert_output --partial "env=[..] tpl=[..]"
}
