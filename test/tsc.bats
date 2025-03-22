setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "tsc" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/tsc.pkl"
linters {
    ["tsc"] = new tsc.Tsc {}
}
EOF
    mkdir -p {a,b}/src
    echo '{"extends": "@tsconfig/node22/tsconfig.json", "compilerOptions": {"outDir": "dist"}, "include": ["src/**/*.ts"]}' > a/tsconfig.json
    echo '{"extends": "@tsconfig/node22/tsconfig.json", "compilerOptions": {"outDir": "dist"}, "include": ["src/**/*.ts"]}' > b/tsconfig.json
    echo "const x: number = 'hello';" > a/src/test.ts
    echo "const y: string = 1;" > b/src/test.ts
    git add a b
    run hk check -v
    assert_failure
    assert_output --partial 'xxx'
}
