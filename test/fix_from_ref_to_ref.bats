#!/usr/bin/env mise run test:bats

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

