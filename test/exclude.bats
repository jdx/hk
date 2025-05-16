setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "exclude" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js", "*.ts")
                exclude = List("*.test.js", "*.test.ts")
                check = "prettier --check {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"
    # Create files that should be checked
    echo "console.log('test1')" > test1.js
    echo "console.log('test2')" > test2.ts
    
    # Create files that should be excluded
    echo "console.log('test3')" > test3.test.js
    echo "console.log('test4')" > test4.test.ts
    
    git add test1.js test2.ts test3.test.js test4.test.ts
    run hk check -v
    assert_failure
    assert_output --partial 'DEBUG $ prettier --check test1.js test2.ts
'
    assert_output --partial '[warn] Code style issues found in 2 files. Run Prettier with --write to fix.'
}

@test "exclude with dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/prettier.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                dir = "src"
                glob = List("*.js", "*.ts")
                exclude = List("*.test.js", "*.test.ts")
                check = "prettier --check {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"
    mkdir -p src
    # Create files that should be checked
    echo "console.log('test1')" > src/test1.js
    echo "console.log('test2')" > src/test2.ts
    
    # Create files that should be excluded
    echo "console.log('test3')" > src/test3.test.js
    echo "console.log('test4')" > src/test4.test.ts
    
    # Create files outside the dir that should be ignored
    echo "console.log('test5')" > test5.js
    echo "console.log('test6')" > test6.ts
    
    git add src/test1.js src/test2.ts src/test3.test.js src/test4.test.ts test5.js test6.ts
    run hk check -v
    assert_failure
    assert_output --partial 'DEBUG $ prettier --check test1.js test2.ts
'
    assert_output --partial '[warn] Code style issues found in 2 files. Run Prettier with --write to fix.'
} 
