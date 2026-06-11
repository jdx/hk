#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "group inherited dir and prefix apply to child steps" {
    mkdir -p packages/frontend different/path
    echo "console.log('frontend')" > packages/frontend/app.js
    echo "console.log('override')" > different/path/app.js
    echo "{}" > packages/frontend/package.json
    echo "{}" > different/path/package.json
    git add packages/frontend/app.js packages/frontend/package.json different/path/app.js different/path/package.json
    git commit -m "add test files"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["frontend"] = new Group {
                dir = "packages/frontend"
                prefix = "env GROUP_PREFIX=1"
                workspace_indicator = "package.json"
                shell = "bash -o errexit -c"
                stage = "dist/**"
                exclude = List("**/*.snap")
                steps {
                    ["prettier"] {
                        check = "sh -c 'printf %s \"\$GROUP_PREFIX\" > {{root}}/inherited-prefix.txt; pwd > {{root}}/inherited-dir.txt'"
                        glob = "*.js"
                        fix = "true"
                    }
                    ["eslint"] {
                        dir = "different/path"
                        check = "sh -c 'printf %s \"\$GROUP_PREFIX\" > {{root}}/override-prefix.txt; pwd > {{root}}/override-dir.txt'"
                        glob = "*.js"
                        fix = "true"
                    }
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_success

    assert_file_contains inherited-prefix.txt "1"
    assert_file_contains inherited-dir.txt "/packages/frontend"
    assert_file_contains override-prefix.txt "1"
    assert_file_contains override-dir.txt "/different/path"
}

@test "group inherited exclude filters child step files" {
    mkdir -p packages/frontend
    echo "console.log('frontend')" > packages/frontend/app.js
    echo "snapshot" > packages/frontend/app.snap
    git add packages/frontend/app.js packages/frontend/app.snap
    git commit -m "add test files"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["frontend"] = new Group {
                dir = "packages/frontend"
                exclude = List("**/*.snap")
                steps {
                    ["list-files"] {
                        check = "printf '%s\\n' {{files}} > {{root}}/seen-files.txt"
                        glob = List("*.js", "*.snap")
                    }
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_success

    assert_file_contains seen-files.txt "app.js"
    assert_file_not_contains seen-files.txt "app.snap"
}

@test "group inherited stage stages child fix output" {
    mkdir -p packages/frontend/dist/assets
    echo "console.log('frontend')" > packages/frontend/app.js
    echo "old" > packages/frontend/dist/assets/output.txt
    git add packages/frontend/app.js packages/frontend/dist/assets/output.txt
    git commit -m "add test files"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["fix"] {
        fix = true
        steps {
            ["frontend"] = new Group {
                dir = "packages/frontend"
                stage = "dist/**"
                steps {
                    ["build"] {
                        glob = "dist/assets/*.txt"
                        fix = "printf generated > dist/assets/output.txt"
                    }
                }
            }
        }
    }
}
EOF

    echo "dirty" > packages/frontend/dist/assets/output.txt

    run hk run fix
    assert_success

    run git diff --name-only --cached
    assert_success
    assert_output --partial "packages/frontend/dist/assets/output.txt"
}
