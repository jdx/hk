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
import "$PKL_PATH/builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["tsc"] = builtins.tsc
        }
    }
}
EOF
    mkdir -p {a,b}/src
    echo '{"compilerOptions": {"outDir": "dist"}, "include": ["src/**/*.ts"]}' > a/tsconfig.json
    echo '{"compilerOptions": {"outDir": "dist"}, "include": ["src/**/*.ts"]}' > b/tsconfig.json
    echo "const x: number = 'hello';" > a/src/test.ts
    echo "const y: number = 1;" > b/src/test.ts
    git add a b
    run hk check -v
    assert_failure
    assert_output --partial "a/src/test.ts(1,7): error TS2322: Type 'string' is not assignable to type 'number'."
}
