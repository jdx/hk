#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk --version prints version" {
    run hk --version
    assert_output --regexp "^hk\ [0-9]+\.[0-9]+\.[0-9]+$"
}

@test "hk generate creates hk.pkl" {
    hk g
    assert_file_contains hk.pkl "min_hk_version"
}

@test "hk install creates git hooks" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
linters { ["prettier"] = new prettier.Prettier {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    hk install
    assert_file_exists ".git/hooks/pre-commit"
}

@test "git runs pre-commit on staged files" {
    cat <<EOF > test.js
console.log("test")
EOF
    run git add test.js
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
linters { ["prettier"] = new prettier.Prettier {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    hk install
    run cat test.js
    assert_output 'console.log("test")'
    git commit -m "test"
    run cat test.js
    assert_output 'console.log("test");'
}

@test "hk run pre-commit --all runs on all files" {
    cat <<EOF > test.js
console.log("test")
EOF
    git add test.js
    git commit -m init
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
linters { ["prettier"] = new prettier.Prettier {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    hk run pre-commit -a
    run cat test.js
    assert_output 'console.log("test");'
}

@test "builtin: json" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/jq.pkl"
linters { ["jq"] = new jq.Jq {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    cat <<EOF > test.json
{ "invalid": 
EOF
    git add test.json
    run hk run pre-commit
    assert_failure
    assert_output --partial "jq: parse error"
}

@test "builtin: json format" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/jq.pkl"
linters { ["jq"] = new jq.Jq {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    cat <<EOF > test.json
{"test": 123}
EOF
    git add test.json
    hk run pre-commit
    assert_file_contains test.json '{
  "test": 123
}'
}

@test "builtin: yaml" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/yq.pkl"
linters { ["yq"] = new yq.Yq {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    cat <<EOF > test.yaml
test: :
EOF
    git add test.yaml
    run hk run pre-commit
    assert_failure
    assert_output --partial "yaml: mapping values are not allowed"
}

@test "builtin: yaml format" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/yq.pkl"
linters { ["yq"] = new yq.Yq {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    cat <<EOF > test.yaml
    test: 123
EOF
    git add test.yaml
    cat test.yaml
    hk run pre-commit
    assert_file_contains test.yaml 'test: 123'
}

@test "builtin: shellcheck" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/shellcheck.pkl"
linters { ["shellcheck"] = new shellcheck.Shellcheck {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    cat <<EOF > test.sh
#!/bin/bash
cat \$1
EOF
    git add test.sh
    run hk run pre-commit
    assert_failure
    assert_output --partial "SC2086"
}

@test "HK_SKIP_STEPS skips specified steps" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
import "$PKL_PATH/builtins/shellcheck.pkl"
linters {
    ["prettier"] = new prettier.Prettier {}
    ["shellcheck"] = new shellcheck.Shellcheck {}
}
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    touch test.sh
    touch test.js
    git add test.sh test.js
    export HK_SKIP_STEPS="shellcheck"
    run hk run pre-commit -v
    assert_success
    assert_output --partial "prettier"
    assert_output --partial "shellcheck: skipping step due to HK_SKIP_STEPS"
}

@test "HK_SKIP_HOOK skips entire hooks" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
import "$PKL_PATH/builtins/shellcheck.pkl"
linters {
    ["prettier"] = new prettier.Prettier {}
    ["shellcheck"] = new shellcheck.Shellcheck {}
}
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    touch test.sh
    touch test.js
    git add test.sh test.js
    export HK_SKIP_HOOK="pre-commit"
    run hk run pre-commit -v
    assert_success
    assert_output --partial "pre-commit: skipping hook due to HK_SKIP_HOOK"
}

@test "check_first waits" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
linters {
    ["a"] {
        glob = List("*.sh")
        check = "echo 'start a' && sleep 0.1 && echo 'exit a' && exit 1"
        fix = "echo 'start a' && sleep 0.1 && echo 'end a'"
    }
    ["b"] {
        glob = List("*.sh")
        check = "echo 'start b' && echo 'exit b' && exit 1"
        fix = "echo 'start b' && echo 'end b' && touch test.sh"
    }
}
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
EOF
    touch test.sh
    git add test.sh
    run hk run pre-commit -v
    assert_success

    # runs b to completion without a
    assert_output --partial "INFO  b               1 file – *.sh – echo 'start b' && echo 'end b' && touch test.sh
DEBUG $ echo 'start b' && echo 'end b' && touch test.sh
INFO  b               start b
INFO  b               end b
INFO  b             ✓ 1 file modified"
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
import "$PKL_PATH/builtins/prettier.pkl"
linters { ["prettier"] = new prettier.Prettier {} }
hooks { ["pre-commit"] = new { ["fix"] = new Fix {} } }
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
