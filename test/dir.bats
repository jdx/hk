setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] = new LinterStep {
                dir = "ui"
                glob = List("*.html", "*.ts")
                check = "prettier --check {{files}}"
            }
        }
    }
}
EOF
    mkdir -p ui/subdir
    echo "<html><body>test</body></html>" > ui/subdir/test.html
    echo "console.log('test')" > ui/test.ts
    echo "console.log('test')" > root.ts
    git add ui/subdir/test.html ui/test.ts root.ts
    run hk check -v
    assert_failure
    assert_output --partial '[warn] subdir/test.html'
    assert_output --partial '[warn] test.ts'
    assert_output --partial '[warn] Code style issues found in 2 files. Run Prettier with --write to fix.'
}
