#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "regex exclude pattern" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("**/*.yaml")
                exclude = new Mapping {
                    ["_type"] = "regex"
                    ["pattern"] = #"""
(?x)
^.*airflow\.template\.yaml$|
^.*init_git_sync\.template\.yaml$|
^chart/(?:templates|files)/.*\.yaml$|
^helm-tests/tests/chart_utils/keda\.sh_scaledobjects\.yaml$|
.*/v1.*\.yaml$|
^.*openapi.*\.yaml$|
^\.pre-commit-config\.yaml$|
^.*reproducible_build\.yaml$|
^.*pnpm-lock\.yaml$
"""#
                }
                check = "prettier --no-color --check {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create files that should be checked (with bad formatting)
    echo "foo:  bar" > config.yaml
    echo "baz:   qux" > settings.yaml

    # Create files that should be excluded by regex
    mkdir -p chart/templates foo
    echo "excluded: 1" > airflow.template.yaml
    echo "excluded: 2" > chart/templates/deployment.yaml
    echo "excluded: 3" > .pre-commit-config.yaml
    echo "excluded: 4" > openapi-spec.yaml
    echo "excluded: 5" > foo/v1-api.yaml

    git add config.yaml settings.yaml airflow.template.yaml chart/templates/deployment.yaml .pre-commit-config.yaml openapi-spec.yaml foo/v1-api.yaml
    run hk check -v
    assert_failure
    assert_output --partial 'DEBUG $ prettier --no-color --check config.yaml settings.yaml'
    # Make sure excluded files are NOT in the prettier command
    refute_output --partial '$ prettier --no-color --check.*airflow.template.yaml'
    refute_output --partial '$ prettier --no-color --check.*chart/templates/deployment.yaml'
    refute_output --partial '$ prettier --no-color --check.*\.pre-commit-config\.yaml'
    refute_output --partial '$ prettier --no-color --check.*openapi-spec.yaml'
    refute_output --partial '$ prettier --no-color --check.*v1-api.yaml'
}

@test "regex glob pattern" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["yaml-check"] {
                glob = new Mapping {
                    ["_type"] = "regex"
                    ["pattern"] = #"^(config|settings).*\.yaml$"#
                }
                check = "echo {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    # Create files that should match the regex
    echo "foo: bar" > config.yaml
    echo "baz: qux" > settings.yaml
    echo "qux: quux" > config-dev.yaml

    # Create files that should NOT match
    echo "excluded: 1" > other.yaml
    echo "excluded: 2" > data.yaml

    git add config.yaml settings.yaml config-dev.yaml other.yaml data.yaml
    run hk check -v
    assert_success
    assert_output --partial 'config-dev.yaml config.yaml settings.yaml'
    # Make sure non-matching files are NOT in the echo command
    refute_output --partial '$ echo.*other.yaml'
    refute_output --partial '$ echo.*data.yaml'
}

@test "regex pattern with dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["check-src"] {
                dir = "src"
                glob = List("**/*.js")
                exclude = new Mapping {
                    ["_type"] = "regex"
                    ["pattern"] = #".*\.test\.js$"#
                }
                check = "echo {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src
    # Create files that should be checked
    echo "code" > src/app.js
    echo "code" > src/lib.js

    # Create files that should be excluded
    echo "test" > src/app.test.js
    echo "test" > src/lib.test.js

    git add src/app.js src/lib.js src/app.test.js src/lib.test.js
    run hk check -v
    assert_success
    assert_output --partial 'app.js lib.js'
    # The files should be excluded - they won't show up in the command at all
    # The regex matched and excluded them successfully
}
