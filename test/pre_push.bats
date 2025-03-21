setup() {
    load 'test_helper/common_setup'
    _common_setup
    TEST_REPO_DIR="$(temp_make)"
    pushd "$TEST_REPO_DIR"
    git init --bare
    popd
    git remote add origin "$TEST_REPO_DIR"
}
teardown() {
    _common_teardown
    chmod -R u+w "$TEST_REPO_DIR"
    temp_del "$TEST_REPO_DIR"
}

@test "pre-push hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
linters { ["prettier"] = new prettier.Prettier {} }
hooks { ["pre-push"] = new { ["check"] = new Check {} } }
EOF
    git add hk.pkl
    git commit -m "install hk"
    git push origin main
    hk install
    echo 'console.log("test")' > test.js
    git add test.js
    git commit -m "test"
    HK_LOG=trace run git push origin main
    assert_failure
    assert_output --partial "[warn] test.js"
}
