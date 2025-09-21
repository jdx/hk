#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "top-level exclude - single pattern as string" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
exclude = "*.test.js"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js", "*.ts")
                check = "prettier --no-color --check {{files}}"
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

    # Create files that should be excluded by top-level exclude
    echo "console.log('test3')" > test3.test.js

    git add test1.js test2.ts test3.test.js
    run hk check -v
    assert_failure
    # Should only check test1.js and test2.ts, not test3.test.js
    assert_output --partial 'DEBUG $ prettier --no-color --check test1.js test2.ts
'
    refute_output --partial 'test3.test.js'
    assert_output --partial '[warn] Code style issues found in 2 files.'
}

@test "top-level exclude - single pattern as list" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
exclude = List("*.test.js")
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js", "*.ts")
                check = "prettier --no-color --check {{files}}"
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

    # Create files that should be excluded by top-level exclude
    echo "console.log('test3')" > test3.test.js

    git add test1.js test2.ts test3.test.js
    run hk check -v
    assert_failure
    # Should only check test1.js and test2.ts, not test3.test.js
    assert_output --partial 'DEBUG $ prettier --no-color --check test1.js test2.ts
'
    refute_output --partial 'test3.test.js'
    assert_output --partial '[warn] Code style issues found in 2 files.'
}

@test "top-level exclude - list of patterns" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
exclude = List("*.test.js", "*.spec.ts", "build/*")
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js", "*.ts")
                check = "prettier --no-color --check {{files}}"
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

    # Create files that should be excluded by different patterns
    echo "console.log('test3')" > test3.test.js
    echo "console.log('test4')" > test4.spec.ts
    mkdir -p build
    echo "console.log('test5')" > build/bundle.js

    git add test1.js test2.ts test3.test.js test4.spec.ts build/bundle.js
    run hk check -v
    assert_failure
    # Should only check test1.js and test2.ts
    assert_output --partial 'DEBUG $ prettier --no-color --check test1.js test2.ts
'
    refute_output --partial 'test3.test.js'
    refute_output --partial 'test4.spec.ts'
    refute_output --partial 'build/bundle.js'
    assert_output --partial '[warn] Code style issues found in 2 files.'
}

@test "top-level exclude combined with step-level exclude" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
exclude = List("*.test.js")
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js", "*.ts")
                exclude = List("*.spec.ts")
                check = "prettier --no-color --check {{files}}"
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

    # Create files that should be excluded by top-level exclude
    echo "console.log('test3')" > test3.test.js

    # Create files that should be excluded by step-level exclude
    echo "console.log('test4')" > test4.spec.ts

    git add test1.js test2.ts test3.test.js test4.spec.ts
    run hk check -v
    assert_failure
    # Should only check test1.js and test2.ts
    assert_output --partial 'DEBUG $ prettier --no-color --check test1.js test2.ts
'
    # Top-level exclude should completely remove files from processing
    refute_output --partial 'test3.test.js'
    # Step-level exclude should remove files from the prettier command but they may appear in file listings
    refute_output --partial 'prettier --no-color --check test1.js test2.ts test4.spec.ts'
    assert_output --partial '[warn] Code style issues found in 2 files.'
}

@test "top-level exclude with directory pattern" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
exclude = List("node_modules", "dist")
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.js", "*.ts")
                check = "prettier --no-color --check {{files}}"
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

    # Create files that should be excluded (in excluded directories)
    mkdir -p node_modules dist
    echo "console.log('test3')" > node_modules/test3.js
    echo "console.log('test4')" > dist/test4.js

    git add test1.js test2.ts node_modules/test3.js dist/test4.js
    run hk check -v
    assert_failure
    # Should only check test1.js and test2.ts
    assert_output --partial 'DEBUG $ prettier --no-color --check test1.js test2.ts
'
    refute_output --partial 'node_modules/test3.js'
    refute_output --partial 'dist/test4.js'
    assert_output --partial '[warn] Code style issues found in 2 files.'
}
