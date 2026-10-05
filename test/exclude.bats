#!/usr/bin/env bats

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

    # Create files that should be excluded
    echo "console.log('test3')" > test3.test.js
    echo "console.log('test4')" > test4.test.ts

    git add test1.js test2.ts test3.test.js test4.test.ts
    run hk check -v
    assert_failure
    assert_output --partial 'DEBUG $ prettier --no-color --check test1.js test2.ts
'
    assert_output --partial '[warn] Code style issues found in 2 files.'
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
                check = "prettier --no-color --check {{files}}"
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
    assert_output --partial 'DEBUG $ prettier --no-color --check test1.js test2.ts
'
    assert_output --partial '[warn] Code style issues found in 2 files.'
}


@test "exclude with leading dot" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["check"] {
        steps {
            ["error"] {
                glob = "*.txt"
                exclude = List("file.txt")
                check = "echo {{ files }} && exit 1"
            }
        }
    }
}
EOF
    touch file.txt

    run hk check --all
    assert_success

    run hk check ./file.txt
    assert_success

    run hk check .
    assert_success
}

write_directory_exclude_config() {
    local step_exclude=$1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["list"] {
                glob = "**/*.txt"
                $step_exclude
                check = "echo FILES {{ files }}"
            }
        }
    }
}
EOF
    mkdir -p vendor/deep src/vendor
    echo a > keep.txt
    echo b > vendor/a.txt
    echo c > vendor/deep/b.txt
    echo d > src/vendor/c.txt
    echo e > src/main.txt
    git add .
}

@test "step exclude naming a directory excludes the files inside it" {
    write_directory_exclude_config 'exclude = List("vendor")'

    run hk check --all
    assert_success
    assert_output --partial "keep.txt"
    assert_output --partial "src/main.txt"
    # Only the top-level vendor directory is named; src/vendor is not.
    assert_output --partial "src/vendor/c.txt"
    refute_output --partial "vendor/a.txt"
    refute_output --partial "vendor/deep/b.txt"
}

@test "step exclude with a trailing slash or a leading **/ excludes the directory" {
    write_directory_exclude_config 'exclude = List("vendor/")'
    run hk check --all
    assert_success
    assert_output --partial "src/vendor/c.txt"
    refute_output --partial "vendor/a.txt"

    write_directory_exclude_config 'exclude = List("**/vendor")'
    run hk check --all
    assert_success
    assert_output --partial "keep.txt"
    refute_output --partial "vendor/a.txt"
    refute_output --partial "vendor/deep/b.txt"
    refute_output --partial "src/vendor/c.txt"
}

@test "group exclude naming a directory excludes the files inside it" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["group"] = new Group {
                exclude = List("vendor")
                steps {
                    ["list"] {
                        glob = "**/*.txt"
                        check = "echo FILES {{ files }}"
                    }
                }
            }
        }
    }
}
EOF
    mkdir -p vendor/deep
    echo a > keep.txt
    echo b > vendor/a.txt
    echo c > vendor/deep/b.txt
    git add .

    run hk check --all
    assert_success
    assert_output --partial "keep.txt"
    refute_output --partial "vendor/a.txt"
    refute_output --partial "vendor/deep/b.txt"
}

@test "global --exclude with a trailing slash or a leading **/ excludes the directory" {
    write_directory_exclude_config ''

    run hk check --all --exclude "vendor/"
    assert_success
    assert_output --partial "src/vendor/c.txt"
    refute_output --partial "vendor/a.txt"
    refute_output --partial "vendor/deep/b.txt"

    run hk check --all --exclude "**/vendor"
    assert_success
    assert_output --partial "keep.txt"
    refute_output --partial "vendor/a.txt"
    refute_output --partial "src/vendor/c.txt"
}
